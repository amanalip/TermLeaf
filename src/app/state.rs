use std::{
    collections::HashMap,
    mem::size_of,
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{
    document::{
        Document, DocumentError, ImageResource, ImageResourceError, ResourceProvider,
        ResourceReadError,
    },
    layout::PageLayout,
    reader::{self, Mode},
    terminal_image::{
        CellColorMode, CellRenderError, HalfBlockCell, HalfBlockFrame, ImageBackend,
        ImageCapabilities, ImageId, NativeEncodeError, NativeImage, Rgb, encode_native_image,
        failure_caption, render_decoded_half_blocks, select_backend,
    },
    ui::status::StatusMessage,
    ui::theme::{ColorMode, ThemeName},
};
use anyhow::{Context, Result, bail};

use super::{
    Action,
    worker::{Generation, SubmitError, TaskError, WorkerCoordinator},
};
use crate::document::model::Position;

/// One open book plus its logical reading state.
///
/// The layout cache is keyed by content width; navigation between draws
/// reuses it, and a resize replaces it wholesale. The cache owner is the
/// session itself and the invalidation rule is the width key.
#[derive(Debug)]
pub struct ReaderSession {
    document: Document,
    anchor: Position,
    navigation_index: Option<usize>,
    mode: Mode,
    cached_layout: Option<(u16, PageLayout)>,
    resources: ResourceProvider,
    images: HashMap<ImageKey, ImageState>,
    image_ids: HashMap<ImageKey, ImageId>,
    next_image_id: u32,
    image_workers: WorkerCoordinator<ImageJob, ImageOutput, ImageJobError>,
    observed_dropped_completions: u64,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct ImageKey {
    section: usize,
    block: usize,
    reference: String,
}

#[derive(Debug)]
enum ImageState {
    Loading {
        generation: Generation,
    },
    Ready {
        generation: Generation,
        image: Arc<PreparedImage>,
    },
    Failed {
        generation: Generation,
        caption: String,
    },
}

#[derive(Clone, Debug)]
struct ImageJob {
    key: ImageKey,
    id: ImageId,
    provider: ResourceProvider,
    resource: ImageResource,
    columns: u16,
    rows: u16,
    background: Rgb,
    backend: ImageBackend,
    declared: Option<DeclaredImageFormat>,
}

#[derive(Clone, Copy, Debug)]
enum DeclaredImageFormat {
    Raster(image::ImageFormat),
    Vector(crate::document::VectorFormat),
}

#[derive(Debug)]
struct ImageOutput {
    key: ImageKey,
    image: PreparedImage,
}

#[derive(Debug)]
enum PreparedImage {
    Cells(Arc<HalfBlockFrame>),
    Native(Arc<NativeImage>),
}

#[derive(Debug, thiserror::Error)]
#[error("{reason}")]
struct ImageJobError {
    key: ImageKey,
    dimensions: Option<(u32, u32)>,
    reason: ImageFailure,
}

#[derive(Debug, thiserror::Error)]
enum ImageFailure {
    #[error("read: {0}")]
    Read(ResourceReadError),
    #[error("decode: {0}")]
    Decode(ImageResourceError),
    #[error("render: {0}")]
    Render(CellRenderError),
    #[error("native transport: {0}")]
    Native(NativeEncodeError),
}

impl ImageFailure {
    fn short_reason(&self) -> String {
        match self {
            Self::Read(reason) => format!("read: {reason}"),
            Self::Decode(ImageResourceError::UnsupportedFormat) => {
                "decode: unsupported format".to_owned()
            }
            Self::Decode(ImageResourceError::DimensionTooLarge { .. }) => {
                "decode: dimension limit".to_owned()
            }
            Self::Decode(ImageResourceError::TooManyPixels { .. }) => {
                "decode: pixel limit".to_owned()
            }
            Self::Decode(ImageResourceError::AllocationTooLarge { .. }) => {
                "decode: allocation limit".to_owned()
            }
            Self::Decode(ImageResourceError::InputTooLarge { .. }) => {
                "decode: input limit".to_owned()
            }
            Self::Decode(ImageResourceError::DecodeFailed { .. }) => {
                "decode: corrupt data".to_owned()
            }
            Self::Decode(ImageResourceError::Vector(_)) => "decode: vector rejected".to_owned(),
            Self::Render(reason) => format!("render: {reason}"),
            Self::Native(reason) => format!("native transport: {reason}"),
        }
    }
}

/// One image overlay projected into the current content viewport.
#[derive(Clone, Debug)]
pub struct ImageOverlay {
    pub row: u16,
    pub visual: ImageVisual,
}

/// Paintable current state of one visible image placeholder.
#[derive(Clone, Debug)]
pub enum ImageVisual {
    Loading(String),
    ReadyCells(Arc<HalfBlockFrame>),
    Native(Arc<NativeImage>),
    Failed(String),
}

impl ReaderSession {
    /// Opens a parsed document anchored at its start in paged mode.
    pub fn new(document: Document, resources: ResourceProvider) -> std::io::Result<Self> {
        let anchor = document.first_position().unwrap_or(Position::ORIGIN);
        let navigation_index = document
            .navigation_points()
            .iter()
            .position(|point| point.position() == anchor);
        let image_workers = WorkerCoordinator::new(process_image, image_output_size)?;
        Ok(Self {
            document,
            anchor,
            navigation_index,
            mode: Mode::Paged,
            cached_layout: None,
            resources,
            images: HashMap::new(),
            image_ids: HashMap::new(),
            next_image_id: 1,
            image_workers,
            observed_dropped_completions: 0,
        })
    }

    /// The open document.
    #[must_use]
    pub const fn document(&self) -> &Document {
        &self.document
    }

    /// The validated logical reading anchor.
    #[must_use]
    pub const fn anchor(&self) -> Position {
        self.anchor
    }

    /// The active reading mode.
    #[must_use]
    pub const fn mode(&self) -> Mode {
        self.mode
    }

    /// Sets the reading mode; the anchor never moves on a mode switch.
    pub const fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
    }

    /// The layout for one content width, reusing the cache when possible.
    #[must_use]
    pub fn layout_for(&mut self, width: u16) -> &PageLayout {
        let stale = self
            .cached_layout
            .as_ref()
            .is_none_or(|(cached, _)| *cached != width);
        if stale {
            let layout = crate::layout::layout_document(&self.document, width);
            self.cached_layout = Some((width, layout));
        }
        &self
            .cached_layout
            .as_ref()
            .expect("layout cache is populated immediately above")
            .1
    }

    /// The cached layout when it already matches `width`.
    #[must_use]
    pub const fn cached_layout(&self, width: u16) -> Option<&PageLayout> {
        match self.cached_layout.as_ref() {
            Some((cached, layout)) if *cached == width => Some(layout),
            _ => None,
        }
    }

    /// Visible row cells for one content viewport, warming the cache.
    ///
    /// Cells carry plain text plus the inline role; the UI layer styles them.
    #[must_use]
    pub fn plan_rows(
        &mut self,
        width: u16,
        height: u16,
    ) -> Vec<Vec<crate::layout::viewport::RowCell>> {
        let stale = self
            .cached_layout
            .as_ref()
            .is_none_or(|(cached, _)| *cached != width);
        if stale {
            let layout = crate::layout::layout_document(&self.document, width);
            self.cached_layout = Some((width, layout));
        }
        let cache = self
            .cached_layout
            .as_ref()
            .expect("layout cache is populated immediately above");
        let anchor = self.anchor.absolute_byte(&self.document);
        crate::layout::viewport::viewport_row_texts(&self.document, &cache.1, anchor, height)
    }

    /// Applies one navigation step, keeping the previous anchor on failure.
    pub fn navigate<F>(&mut self, content_width: u16, step: F)
    where
        F: FnOnce(&Document, &PageLayout, Position) -> Option<Position>,
    {
        let stale = self
            .cached_layout
            .as_ref()
            .is_none_or(|(cached, _)| *cached != content_width);
        if stale {
            let layout = crate::layout::layout_document(&self.document, content_width);
            self.cached_layout = Some((content_width, layout));
        }
        let cache = self
            .cached_layout
            .as_ref()
            .expect("layout cache is populated immediately above");
        if let Some(next) = step(&self.document, &cache.1, self.anchor)
            && next != self.anchor
        {
            self.anchor = next;
            self.roll_image_generation();
        }
    }

    fn navigate_section(&mut self, direction: reader::Direction) {
        if let Some((index, position)) =
            reader::step_section(&self.document, self.navigation_index, direction)
        {
            self.navigation_index = Some(index);
            if position != self.anchor {
                self.anchor = position;
                self.roll_image_generation();
            }
        }
    }

    fn jump_to_navigation(&mut self, index: usize) {
        let Some(position) = self
            .document
            .navigation_points()
            .get(index)
            .map(crate::document::NavigationPoint::position)
        else {
            return;
        };
        self.navigation_index = Some(index);
        if position != self.anchor {
            self.anchor = position;
            self.roll_image_generation();
        }
    }

    fn roll_image_generation(&mut self) {
        if self.image_workers.next_generation().is_ok() {
            self.images.clear();
            self.observed_dropped_completions = self.image_workers.stats().dropped_completions;
        }
    }

    fn resized(&mut self) {
        self.roll_image_generation();
    }

    /// Drains all currently available worker completions without blocking.
    pub fn drain_image_work(&mut self) {
        while let Ok(Some(completion)) = self.image_workers.try_recv() {
            let generation = completion.generation;
            match completion.result {
                Ok(output) => {
                    if matches!(
                        self.images.get(&output.key),
                        Some(ImageState::Loading { generation: expected }) if *expected == generation
                    ) {
                        self.images.insert(
                            output.key,
                            ImageState::Ready {
                                generation,
                                image: Arc::new(output.image),
                            },
                        );
                    }
                }
                Err(TaskError::Decode(error)) => {
                    let caption = self.failure_for(
                        &error.key,
                        error.dimensions,
                        &error.reason.short_reason(),
                    );
                    if matches!(
                        self.images.get(&error.key),
                        Some(ImageState::Loading { generation: expected }) if *expected == generation
                    ) {
                        self.images.insert(
                            error.key,
                            ImageState::Failed {
                                generation,
                                caption,
                            },
                        );
                    }
                }
                Err(TaskError::Panicked) => {
                    for state in self.images.values_mut() {
                        if matches!(state, ImageState::Loading { generation: expected } if *expected == generation)
                        {
                            *state = ImageState::Failed {
                                generation,
                                caption: "[image: worker failed]".to_owned(),
                            };
                        }
                    }
                }
                Err(TaskError::Cancelled) => {}
            }
        }
        let dropped = self.image_workers.stats().dropped_completions;
        if dropped > self.observed_dropped_completions {
            let generation = self.image_workers.generation();
            let keys = self
                .images
                .iter()
                .filter(|(_, state)| {
                    matches!(state, ImageState::Loading { generation: expected } if *expected == generation)
                })
                .map(|(key, _)| key.clone())
                .collect::<Vec<_>>();
            for key in keys {
                let caption = self.failure_for(&key, None, "worker output dropped");
                self.images.insert(
                    key,
                    ImageState::Failed {
                        generation,
                        caption,
                    },
                );
            }
        }
        self.observed_dropped_completions = dropped;
    }

    /// Submits visible placeholders and returns their current paint state.
    pub fn prepare_visible_images(
        &mut self,
        width: u16,
        height: u16,
        backend: ImageBackend,
        background: Rgb,
    ) -> Vec<ImageOverlay> {
        self.drain_image_work();
        if height == 0 {
            return Vec::new();
        }
        let anchor_byte = self.anchor.absolute_byte(&self.document);
        let locations = {
            let layout = self.layout_for(width);
            let top = layout
                .row_after(anchor_byte)
                .min(layout.rows().len().saturating_sub(1));
            let end = top
                .saturating_add(usize::from(height))
                .min(layout.rows().len());
            let mut locations = Vec::new();
            for index in top..end {
                let row = &layout.rows()[index];
                if index > top
                    && layout.rows()[index - 1].section() == row.section()
                    && layout.rows()[index - 1].block() == row.block()
                {
                    continue;
                }
                let mut overlay_index = index;
                while overlay_index < end {
                    let candidate = &layout.rows()[overlay_index];
                    if candidate.section() != row.section() || candidate.block() != row.block() {
                        break;
                    }
                    if candidate.spans().is_empty() {
                        break;
                    }
                    overlay_index += 1;
                }
                locations.push((
                    u16::try_from(overlay_index - top).unwrap_or(u16::MAX),
                    row.section(),
                    row.block(),
                ));
            }
            locations
        };
        let mut visible = Vec::new();
        for (screen_row, section_index, block_index) in locations {
            if let Some(overlay) = self.prepare_image_overlay(
                screen_row,
                section_index,
                block_index,
                width,
                backend,
                background,
            ) {
                visible.push(overlay);
            }
        }
        visible
    }

    fn prepare_image_overlay(
        &mut self,
        row: u16,
        section: usize,
        block: usize,
        width: u16,
        backend: ImageBackend,
        background: Rgb,
    ) -> Option<ImageOverlay> {
        let block_model = self.document.sections().get(section)?.blocks().get(block)?;
        if block_model.kind() != crate::document::BlockKind::Image {
            return None;
        }
        let resource = block_model
            .resource()
            .cloned()
            .unwrap_or_else(ImageResource::blocked);
        let key = ImageKey {
            section,
            block,
            reference: resource.reference().unwrap_or("blocked").to_owned(),
        };
        if !self.images.contains_key(&key) {
            if let Some(id) = self.image_id(&key) {
                self.submit_image(key.clone(), id, resource, width, backend, background);
            } else {
                let generation = self.image_workers.generation();
                let caption = self.failure_for(&key, None, "image identifier limit reached");
                self.images.insert(
                    key.clone(),
                    ImageState::Failed {
                        generation,
                        caption,
                    },
                );
            }
        }
        let caption = self.image_caption(&key);
        let visual = match self.images.get(&key) {
            Some(ImageState::Loading { .. }) => {
                ImageVisual::Loading(format!("{caption} (loading)"))
            }
            Some(ImageState::Ready { generation, image })
                if *generation == self.image_workers.generation() =>
            {
                match image.as_ref() {
                    PreparedImage::Cells(frame) => ImageVisual::ReadyCells(Arc::clone(frame)),
                    PreparedImage::Native(image) => ImageVisual::Native(Arc::clone(image)),
                }
            }
            Some(ImageState::Failed {
                generation,
                caption,
            }) if *generation == self.image_workers.generation() => {
                ImageVisual::Failed(caption.clone())
            }
            None => ImageVisual::Failed(failure_caption(Some(&caption), None, "not queued")),
            _ => ImageVisual::Failed(failure_caption(Some(&caption), None, "stale result")),
        };
        Some(ImageOverlay { row, visual })
    }

    fn image_id(&mut self, key: &ImageKey) -> Option<ImageId> {
        if let Some(id) = self.image_ids.get(key) {
            return Some(*id);
        }
        let id = ImageId::new(self.next_image_id)?;
        self.next_image_id = self.next_image_id.checked_add(1).unwrap_or(0);
        self.image_ids.insert(key.clone(), id);
        Some(id)
    }

    fn submit_image(
        &mut self,
        key: ImageKey,
        id: ImageId,
        resource: ImageResource,
        width: u16,
        backend: ImageBackend,
        background: Rgb,
    ) {
        let generation = self.image_workers.generation();
        if !resource.is_fetchable() {
            let caption = self.failure_for(&key, None, "blocked resource");
            self.images.insert(
                key,
                ImageState::Failed {
                    generation,
                    caption,
                },
            );
            return;
        }
        if backend == ImageBackend::Caption {
            let caption = self.failure_for(&key, None, "terminal image backend unavailable");
            self.images.insert(
                key,
                ImageState::Failed {
                    generation,
                    caption,
                },
            );
            return;
        }
        let limits = crate::document::ImageLimits::default();
        let charged = resource
            .byte_len()
            .unwrap_or(limits.max_input_bytes)
            .min(limits.max_input_bytes.saturating_add(1));
        let job = ImageJob {
            key: key.clone(),
            id,
            provider: self.resources.clone(),
            resource,
            columns: width,
            rows: u16::try_from(crate::layout::IMAGE_PLACEHOLDER_ROWS).unwrap_or(u16::MAX),
            background,
            backend,
            declared: declared_image_format(&key.reference),
        };
        let bytes = usize::try_from(charged)
            .unwrap_or(usize::MAX)
            .saturating_add(size_of::<ImageJob>())
            .saturating_add(job.key.reference.capacity())
            .saturating_add(job.resource.reference().map_or(0, str::len));
        match self.image_workers.try_submit(generation, job, bytes) {
            Ok(()) => {
                self.images.insert(key, ImageState::Loading { generation });
            }
            Err(error) => {
                let reason = submit_reason(error);
                let caption = self.failure_for(&key, None, reason);
                self.images.insert(
                    key,
                    ImageState::Failed {
                        generation,
                        caption,
                    },
                );
            }
        }
    }

    fn image_caption(&self, key: &ImageKey) -> String {
        self.document
            .block_text(key.section, key.block)
            .unwrap_or("[image]")
            .to_owned()
    }

    fn failure_for(&self, key: &ImageKey, dimensions: Option<(u32, u32)>, reason: &str) -> String {
        let caption = self.image_caption(key);
        let alt = caption
            .strip_prefix("[image:")
            .and_then(|value| value.strip_suffix(']'))
            .map(str::trim);
        failure_caption(alt, dimensions, reason)
    }

    /// Requests cooperative cancellation without waiting for image workers.
    pub fn request_worker_shutdown(&self) {
        self.image_workers.request_shutdown();
    }

    /// Joins image workers after terminal restoration.
    pub fn join_workers(&mut self) {
        self.image_workers.join_workers();
    }
}

fn process_image(
    job: ImageJob,
    token: &super::worker::CancellationToken,
) -> Result<ImageOutput, TaskError<ImageJobError>> {
    let limits = crate::document::ImageLimits::default();
    token.checkpoint()?;
    let bytes = job
        .provider
        .read_bounded(&job.resource, limits.max_input_bytes)
        .map_err(|reason| {
            TaskError::Decode(ImageJobError {
                key: job.key.clone(),
                dimensions: None,
                reason: ImageFailure::Read(reason),
            })
        })?;
    token.checkpoint()?;
    let decoded = decode_image_bytes(&bytes, &limits, job.declared).map_err(|reason| {
        let dimensions = dimensions_from_error(&reason);
        TaskError::Decode(ImageJobError {
            key: job.key.clone(),
            dimensions,
            reason: ImageFailure::Decode(reason),
        })
    })?;
    token.checkpoint()?;
    let image = match job.backend {
        ImageBackend::TrueColorCells | ImageBackend::Ansi256Cells => {
            let mode = if job.backend == ImageBackend::TrueColorCells {
                CellColorMode::TrueColor
            } else {
                CellColorMode::Ansi256
            };
            render_decoded_half_blocks(&decoded, job.columns, job.rows, job.background, mode)
                .map(|frame| PreparedImage::Cells(Arc::new(frame)))
                .map_err(|reason| {
                    TaskError::Decode(ImageJobError {
                        key: job.key.clone(),
                        dimensions: Some((decoded.width(), decoded.height())),
                        reason: ImageFailure::Render(reason),
                    })
                })?
        }
        ImageBackend::Kitty | ImageBackend::Sixel | ImageBackend::Iterm2 => encode_native_image(
            job.backend,
            job.id,
            &decoded,
            job.columns,
            job.rows,
            job.background,
        )
        .map(|image| PreparedImage::Native(Arc::new(image)))
        .map_err(|reason| {
            TaskError::Decode(ImageJobError {
                key: job.key.clone(),
                dimensions: Some((decoded.width(), decoded.height())),
                reason: ImageFailure::Native(reason),
            })
        })?,
        ImageBackend::Caption => {
            return Err(TaskError::Decode(ImageJobError {
                key: job.key,
                dimensions: Some((decoded.width(), decoded.height())),
                reason: ImageFailure::Native(NativeEncodeError::NotNative),
            }));
        }
    };
    token.checkpoint()?;
    Ok(ImageOutput {
        key: job.key,
        image,
    })
}

fn decode_image_bytes(
    bytes: &[u8],
    limits: &crate::document::ImageLimits,
    declared: Option<DeclaredImageFormat>,
) -> Result<crate::document::DecodedImage, ImageResourceError> {
    match declared {
        Some(DeclaredImageFormat::Vector(format))
            if crate::document::vector::sniff_vector_format(bytes).is_none()
                && crate::document::image::sniff_format(bytes).is_err() =>
        {
            crate::document::vector::decode_vector_bounded_with_limits(
                bytes,
                limits,
                &crate::document::VectorLimits::default(),
                Some(format),
            )
            .map_err(ImageResourceError::from)
        }
        Some(DeclaredImageFormat::Raster(format)) => {
            crate::document::image::decode_bounded_with_limits(bytes, limits, Some(format))
        }
        Some(DeclaredImageFormat::Vector(_)) | None => {
            crate::document::image::decode_bounded_with_limits(bytes, limits, None)
        }
    }
}

fn declared_image_format(reference: &str) -> Option<DeclaredImageFormat> {
    let extension = Path::new(reference)
        .extension()?
        .to_str()?
        .to_ascii_lowercase();
    match extension.as_str() {
        "svg" => Some(DeclaredImageFormat::Vector(
            crate::document::VectorFormat::Svg,
        )),
        "svgz" => Some(DeclaredImageFormat::Vector(
            crate::document::VectorFormat::Svgz,
        )),
        extension => image::ImageFormat::from_extension(extension).map(DeclaredImageFormat::Raster),
    }
}

fn image_output_size(output: &ImageOutput) -> usize {
    size_of::<ImageOutput>()
        .saturating_add(output.key.reference.capacity())
        .saturating_add(match &output.image {
            PreparedImage::Cells(frame) => frame
                .cells()
                .len()
                .saturating_mul(size_of::<HalfBlockCell>()),
            PreparedImage::Native(image) => image.allocation_bytes(),
        })
}

const fn dimensions_from_error(error: &ImageResourceError) -> Option<(u32, u32)> {
    match error {
        ImageResourceError::DimensionTooLarge { width, height, .. }
        | ImageResourceError::Vector(
            crate::document::vector::VectorImageError::DimensionTooLarge { width, height, .. },
        ) => Some((*width, *height)),
        _ => None,
    }
}

const fn submit_reason(error: SubmitError) -> &'static str {
    match error {
        SubmitError::QueueFull => "queue full",
        SubmitError::ByteBudgetExceeded { .. } => "worker byte budget exceeded",
        SubmitError::StaleGeneration => "stale generation",
        SubmitError::ShutDown => "workers shut down",
    }
}

impl PartialEq for OpenBook {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

impl Eq for OpenBook {}

#[derive(Clone, Debug)]
pub struct OpenBook {
    path: PathBuf,
}

impl OpenBook {
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum View {
    RecentBooks,
    OpenPath,
    Reader { book: OpenBook },
    LinkFocus,
    TextSelection,
    SearchEntry,
    SearchHistory,
    SearchResults,
    TableOfContents { return_to: Box<View> },
    AnnotationList,
    BookmarkDialog,
    HighlightDialog,
    NoteEditor,
    ThemeSelection { return_to: Box<View> },
    LinkConfirmation,
    Help { return_to: Box<View> },
    RecoverableError,
    TooSmall,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Focus {
    RecentBooks,
    PathField,
    ReadingAnchor,
    Link,
    SelectionEndpoint,
    SearchField,
    SearchHistoryItem,
    SearchResult,
    TableOfContentsItem,
    AnnotationItem,
    BookmarkNameField,
    HighlightColor,
    NoteField,
    ThemeOption,
    ConfirmationAction,
    Help,
    RecoveryAction,
    SuspendedView,
}

/// Minimum usable terminal geometry below which the reader suspends.
pub const MINIMUM_WIDTH: u16 = 20;
pub const MINIMUM_HEIGHT: u16 = 4;

#[derive(Debug)]
pub struct App {
    view: View,
    running: bool,
    theme: ThemeName,
    no_color: bool,
    color_mode: ColorMode,
    image_backend_override: Option<ImageBackend>,
    theme_cursor: usize,
    toc_cursor: usize,
    message: Option<StatusMessage>,
    reader: Option<ReaderSession>,
    content_width: u16,
    content_height: u16,
}

/// Reader launch choices after configuration precedence is applied.
///
/// `book` is the command-line path when one was supplied; `theme` is already
/// resolved (explicit option, then config.toml, then the built-in default).
#[derive(Debug, Default)]
pub struct StartupOptions {
    /// Local book path supplied on the command line, when any.
    pub book: Option<PathBuf>,
    /// Resolved startup theme for the session.
    pub theme: ThemeName,
    /// Preselected image path. Active capability probing is applied separately.
    pub image_backend: Option<ImageBackend>,
}

impl App {
    /// Creates the initial application state for a home or local-book launch.
    ///
    /// The book is decoded before terminal initialization so failures reach
    /// the reader as plain diagnostics on an untouched shell. EPUB sessions
    /// retain their immutable inspected bytes, while Markdown sessions retain
    /// only a canonical local resource root. Color capability is detected once
    /// here so every later draw uses one fixed decision.
    ///
    /// # Errors
    ///
    /// Returns an error when the supplied book path cannot be inspected, is
    /// not a regular file, exceeds the size limit, or does not decode.
    pub fn open(options: StartupOptions) -> Result<Self> {
        let (view, reader) = match options.book {
            Some(path) => {
                Self::validate_path(&path)?;
                let display = crate::document::sanitize_path(&path.display().to_string());
                let text_limits = crate::document::TextLimits::default();
                let archive_limits = crate::document::ArchiveLimits::default();
                let loaded =
                    crate::document::load_book_with_resources(&path, &text_limits, &archive_limits)
                        .map_err(|error| -> anyhow::Error {
                            match error {
                                DocumentError::Read { source, .. } => anyhow::Error::new(source)
                                    .context(format!("could not read '{display}'")),
                                // Typed document errors already name the path,
                                // reason, and recovery; an extra layer would
                                // only bury them.
                                typed => anyhow::Error::new(typed),
                            }
                        })?;
                let book = OpenBook { path };
                let reader = ReaderSession::new(loaded.document, loaded.resources)
                    .context("could not start image workers")?;
                (View::Reader { book }, Some(reader))
            }
            None => (View::RecentBooks, None),
        };

        Ok(Self {
            view,
            running: true,
            theme: options.theme,
            no_color: std::env::var_os("NO_COLOR").is_some_and(|value| !value.is_empty()),
            color_mode: ColorMode::detect(
                env_value("COLORTERM").as_deref(),
                env_value("TERM").as_deref(),
            ),
            image_backend_override: options.image_backend,
            theme_cursor: options.theme as usize,
            toc_cursor: 0,
            message: None,
            reader,
            content_width: MINIMUM_WIDTH,
            content_height: MINIMUM_HEIGHT,
        })
    }

    fn validate_path(path: &Path) -> Result<()> {
        let display = crate::document::sanitize_path(&path.display().to_string());
        let metadata = path.metadata().with_context(|| {
            format!("could not open book '{display}'; check that the path exists and is readable")
        })?;
        if !metadata.is_file() {
            bail!("could not open book '{display}'; the path is not a file");
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            if metadata.permissions().mode() & 0o444 == 0 {
                bail!("could not open book '{display}'; the file is not readable");
            }
        }
        if crate::document::detect_format(path).is_none() {
            use crate::document::DocumentError;

            return Err(anyhow::Error::new(DocumentError::UnsupportedFormat {
                path: display,
            }));
        }
        Ok(())
    }

    #[must_use]
    pub const fn view(&self) -> &View {
        &self.view
    }

    #[must_use]
    pub const fn focus(&self) -> Focus {
        match self.view {
            View::RecentBooks => Focus::RecentBooks,
            View::OpenPath => Focus::PathField,
            View::Reader { .. } => Focus::ReadingAnchor,
            View::LinkFocus => Focus::Link,
            View::TextSelection => Focus::SelectionEndpoint,
            View::SearchEntry => Focus::SearchField,
            View::SearchHistory => Focus::SearchHistoryItem,
            View::SearchResults => Focus::SearchResult,
            View::TableOfContents { .. } => Focus::TableOfContentsItem,
            View::AnnotationList => Focus::AnnotationItem,
            View::BookmarkDialog => Focus::BookmarkNameField,
            View::HighlightDialog => Focus::HighlightColor,
            View::NoteEditor => Focus::NoteField,
            View::ThemeSelection { .. } => Focus::ThemeOption,
            View::LinkConfirmation => Focus::ConfirmationAction,
            View::Help { .. } => Focus::Help,
            View::RecoverableError => Focus::RecoveryAction,
            View::TooSmall => Focus::SuspendedView,
        }
    }

    #[must_use]
    pub const fn is_running(&self) -> bool {
        self.running
    }

    /// The active theme name; `NO_COLOR` sessions still report the choice.
    #[must_use]
    pub const fn theme(&self) -> ThemeName {
        self.theme
    }

    /// Cursor position inside the theme selection list.
    #[must_use]
    pub const fn theme_cursor(&self) -> usize {
        self.theme_cursor
    }

    /// The selected entry while the table of contents overlay is open.
    #[must_use]
    pub const fn toc_cursor(&self) -> usize {
        self.toc_cursor
    }

    /// Whether the session must render without colors (`NO_COLOR`).
    #[must_use]
    pub const fn no_color(&self) -> bool {
        self.no_color
    }

    /// The terminal color capability detected at launch.
    #[must_use]
    pub const fn color_mode(&self) -> ColorMode {
        self.color_mode
    }

    /// Overrides the detected color capability so tests can exercise every
    /// fallback rendering deterministically.
    pub fn set_color_mode(&mut self, mode: ColorMode) {
        if self.color_mode != mode {
            self.color_mode = mode;
            if let Some(reader) = self.reader.as_mut() {
                reader.roll_image_generation();
            }
        }
    }

    /// Injects an already selected image backend without performing probing.
    pub fn set_image_backend(&mut self, backend: Option<ImageBackend>) {
        if self.image_backend_override != backend {
            self.image_backend_override = backend;
            if let Some(reader) = self.reader.as_mut() {
                reader.roll_image_generation();
            }
        }
    }

    /// The reader session, when a book is open.
    #[must_use]
    pub const fn reader(&self) -> Option<&ReaderSession> {
        self.reader.as_ref()
    }

    /// Mutable reader access for the render layer's viewport reporting.
    #[must_use]
    pub fn reader_mut(&mut self) -> Option<&mut ReaderSession> {
        self.reader.as_mut()
    }

    /// The last rendered content viewport, used by navigation between draws.
    #[must_use]
    pub const fn content_viewport(&self) -> (u16, u16) {
        (self.content_width, self.content_height)
    }

    /// Records the content viewport produced by the latest render.
    pub fn set_content_viewport(&mut self, width: u16, height: u16) {
        if (self.content_width, self.content_height) != (width, height)
            && let Some(reader) = self.reader.as_mut()
        {
            reader.resized();
        }
        self.content_width = width;
        self.content_height = height;
    }

    /// Safest image path supported by the launch color capability.
    #[must_use]
    pub fn image_backend(&self) -> ImageBackend {
        let capabilities = ImageCapabilities {
            true_color: if !self.no_color && self.color_mode == ColorMode::TrueColor {
                crate::terminal_image::CapabilityEvidence::Positive
            } else {
                crate::terminal_image::CapabilityEvidence::Absent
            },
            ansi256: if !self.no_color && self.color_mode == ColorMode::Ansi256 {
                crate::terminal_image::CapabilityEvidence::Positive
            } else {
                crate::terminal_image::CapabilityEvidence::Absent
            },
            ..ImageCapabilities::default()
        };
        select_backend(self.image_backend_override, capabilities).unwrap_or(ImageBackend::Caption)
    }

    /// Drains image completions once per event-loop iteration.
    pub fn drain_image_work(&mut self) {
        if let Some(reader) = self.reader.as_mut() {
            reader.drain_image_work();
        }
    }

    /// Requests worker cancellation without waiting for thread exit.
    pub fn request_worker_shutdown(&self) {
        if let Some(reader) = self.reader.as_ref() {
            reader.request_worker_shutdown();
        }
    }

    /// Joins all application-owned workers after terminal restoration.
    pub fn join_workers(&mut self) {
        if let Some(reader) = self.reader.as_mut() {
            reader.join_workers();
        }
    }

    /// Shows a temporary status message replacing lower-priority fields.
    pub fn set_message(&mut self, text: impl Into<String>) {
        self.message = Some(StatusMessage::new(text));
    }

    #[must_use]
    pub const fn message(&self) -> Option<&StatusMessage> {
        self.message.as_ref()
    }

    /// Applies one action to the application state.
    ///
    /// Temporary messages tick once per delivered action, giving them a
    /// deterministic input-driven lifetime.
    ///
    /// # Panics
    ///
    /// Never in practice: the `ShowToc` arm's `expect` guards an invariant
    /// (the Reader view always carries an open book) enforced by the only
    /// constructor that produces that view. All other paths are total.
    pub fn update(&mut self, action: Action) {
        if matches!(self.view, View::ThemeSelection { .. }) {
            self.update_theme_selection(action);
            self.tick_message();
            return;
        }
        if matches!(self.view, View::TableOfContents { .. }) {
            self.update_toc_selection(action);
            self.tick_message();
            return;
        }

        match action {
            Action::Quit => self.running = false,
            Action::ShowHelp if !matches!(self.view, View::Help { .. }) => {
                self.view = View::Help {
                    return_to: Box::new(self.view.clone()),
                };
            }
            Action::Back => {
                let current = std::mem::replace(&mut self.view, View::RecentBooks);
                self.view = match current {
                    View::Help { return_to } => *return_to,
                    other => other,
                };
            }
            Action::ShowThemes => {
                self.theme_cursor = self.theme as usize;
                self.view = View::ThemeSelection {
                    return_to: Box::new(self.view.clone()),
                };
            }
            Action::ShowToc if matches!(self.view, View::Reader { .. }) => {
                self.show_toc();
            }
            // Without an open book there is nothing to navigate: ShowToc
            // falls through inert like every other unmatched action.
            Action::NextLine | Action::PreviousLine if matches!(self.view, View::Reader { .. }) => {
                let direction = match action {
                    Action::NextLine => reader::Direction::TowardEnd,
                    _ => reader::Direction::TowardStart,
                };
                self.step(|document, layout, anchor| {
                    reader::step_line(layout, document, anchor, direction)
                });
            }
            Action::NextPage | Action::PreviousPage if matches!(self.view, View::Reader { .. }) => {
                let direction = match action {
                    Action::NextPage => reader::Direction::TowardEnd,
                    _ => reader::Direction::TowardStart,
                };
                let rows = usize::from(self.content_height);
                self.step(|document, layout, anchor| {
                    reader::step_page(layout, document, anchor, rows, direction)
                });
            }
            Action::DocumentStart if matches!(self.view, View::Reader { .. }) => {
                self.step(|document, layout, _| reader::jump_document_start(layout, document));
                if let Some(session) = self.reader.as_mut() {
                    session.navigation_index =
                        (!session.document.navigation_points().is_empty()).then_some(0);
                }
            }
            Action::DocumentEnd if matches!(self.view, View::Reader { .. }) => {
                self.step(|document, _, _| reader::jump_document_end(document));
                if let Some(session) = self.reader.as_mut() {
                    session.navigation_index =
                        session.document.navigation_points().len().checked_sub(1);
                }
            }
            Action::SectionStart if matches!(self.view, View::Reader { .. }) => {
                if let Some(session) = self.reader.as_mut() {
                    session.navigate_section(reader::Direction::TowardStart);
                }
            }
            Action::SectionEnd if matches!(self.view, View::Reader { .. }) => {
                if let Some(session) = self.reader.as_mut() {
                    session.navigate_section(reader::Direction::TowardEnd);
                }
            }
            Action::SetModePaged | Action::SetModeContinuous
                if matches!(self.view, View::Reader { .. }) =>
            {
                let mode = match action {
                    Action::SetModePaged => Mode::Paged,
                    _ => Mode::Continuous,
                };
                self.set_mode(mode);
            }
            // Reader actions outside the Reader view are intentionally
            // inert, and Confirm has no global meaning yet: overlays such as
            // help must never move the hidden reading anchor.
            _ => {}
        }

        self.tick_message();
    }

    fn show_toc(&mut self) {
        let session = self.reader.as_ref().expect("reader view implies a book");
        let sections = session.document.navigation_points().len();
        self.toc_cursor = session
            .navigation_index
            .unwrap_or(0)
            .min(sections.saturating_sub(1));
        self.view = View::TableOfContents {
            return_to: Box::new(self.view.clone()),
        };
    }

    fn update_theme_selection(&mut self, action: Action) {
        let return_to = match &self.view {
            View::ThemeSelection { return_to } => (**return_to).clone(),
            _ => View::RecentBooks,
        };
        match action {
            Action::NextLine => {
                self.theme_cursor = (self.theme_cursor + 1) % ThemeName::ALL.len();
            }
            Action::PreviousLine => {
                self.theme_cursor =
                    (self.theme_cursor + ThemeName::ALL.len() - 1) % ThemeName::ALL.len();
            }
            Action::Confirm => {
                self.theme = ThemeName::ALL[self.theme_cursor];
                self.view = return_to;
                if let Some(reader) = self.reader.as_mut() {
                    reader.roll_image_generation();
                }
                let label = self.theme.label();
                self.set_message(format!("Theme: {label}"));
            }
            Action::Quit | Action::Back | Action::ShowThemes => {
                self.view = return_to;
            }
            // Help stays reachable from every interactive surface, including
            // overlays; returning restores the theme list exactly.
            Action::ShowHelp => {
                self.view = View::Help {
                    return_to: Box::new(View::ThemeSelection {
                        return_to: Box::new(return_to),
                    }),
                };
            }
            _ => {}
        }
    }

    /// Applies one action while the table of contents overlay is open.
    ///
    /// Up and Down move the section cursor, Confirm jumps the reading anchor
    /// to the selected section start, help stays reachable, and every other
    /// exit restores the invoking view exactly.
    fn update_toc_selection(&mut self, action: Action) {
        let return_to = match &self.view {
            View::TableOfContents { return_to } => (**return_to).clone(),
            _ => View::RecentBooks,
        };
        let sections = self
            .reader
            .as_ref()
            .map_or(0, |session| session.document.navigation_points().len());
        match action {
            Action::NextLine if sections > 0 => {
                self.toc_cursor = (self.toc_cursor + 1).min(sections - 1);
            }
            Action::PreviousLine => {
                self.toc_cursor = self.toc_cursor.saturating_sub(1);
            }
            Action::Confirm if sections > 0 => {
                let target = self.toc_cursor.min(sections - 1);
                self.view = return_to;
                if let Some(session) = self.reader.as_mut() {
                    session.jump_to_navigation(target);
                }
                let label = self
                    .reader
                    .as_ref()
                    .and_then(|session| session.document.navigation_points().get(target))
                    .map_or("Untitled section", crate::document::NavigationPoint::title);
                self.set_message(format!("Jumped: {label}"));
            }
            Action::Quit | Action::Back | Action::ShowToc | Action::ShowThemes => {
                self.view = return_to;
            }
            // Help stays reachable from every interactive surface; returning
            // restores the contents list exactly.
            Action::ShowHelp => {
                self.view = View::Help {
                    return_to: Box::new(View::TableOfContents {
                        return_to: Box::new(return_to),
                    }),
                };
            }
            // Reader navigation stays inert inside the overlay.
            _ => {}
        }
    }

    fn set_mode(&mut self, mode: Mode) {
        let Some(session) = self.reader.as_mut() else {
            return;
        };
        if session.mode() != mode {
            session.set_mode(mode);
            self.set_message(match mode {
                Mode::Paged => "Paged mode",
                Mode::Continuous => "Continuous mode",
            });
        }
    }

    fn step<F>(&mut self, movement: F)
    where
        F: FnOnce(&Document, &PageLayout, Position) -> Option<Position>,
    {
        if self.reader.is_some() {
            let (width, _) = self.content_viewport();
            if let Some(session) = self.reader.as_mut() {
                session.navigate(width, movement);
            }
        }
    }

    fn tick_message(&mut self) {
        if let Some(message) = self.message.as_mut()
            && message.tick()
        {
            self.message = None;
        }
    }
}

fn env_value(key: &str) -> Option<String> {
    std::env::var_os(key).map(|value| value.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app_with_view(view: View) -> App {
        App {
            view,
            running: true,
            theme: ThemeName::Paper,
            no_color: false,
            color_mode: ColorMode::TrueColor,
            image_backend_override: None,
            theme_cursor: ThemeName::Paper as usize,
            toc_cursor: 0,
            message: None,
            reader: None,
            content_width: MINIMUM_WIDTH,
            content_height: MINIMUM_HEIGHT,
        }
    }

    #[test]
    fn app_002_help_returns_to_its_invoking_view_and_focus() -> Result<()> {
        let mut app = App::open(StartupOptions::default())?;

        app.update(Action::ShowHelp);
        assert_eq!(app.focus(), Focus::Help);
        app.update(Action::Back);

        assert_eq!(app.view(), &View::RecentBooks);
        assert_eq!(app.focus(), Focus::RecentBooks);
        Ok(())
    }

    #[test]
    fn app_002_quit_stops_the_state_loop() -> Result<()> {
        let mut app = App::open(StartupOptions::default())?;

        app.update(Action::Quit);

        assert!(!app.is_running());
        Ok(())
    }

    #[test]
    fn app_001_each_view_owns_exactly_one_focus_kind() {
        let views = [
            View::RecentBooks,
            View::OpenPath,
            View::LinkFocus,
            View::TextSelection,
            View::SearchEntry,
            View::SearchHistory,
            View::SearchResults,
            View::TableOfContents {
                return_to: Box::new(View::RecentBooks),
            },
            View::AnnotationList,
            View::BookmarkDialog,
            View::HighlightDialog,
            View::NoteEditor,
            View::ThemeSelection {
                return_to: Box::new(View::RecentBooks),
            },
            View::LinkConfirmation,
            View::RecoverableError,
            View::TooSmall,
        ];
        let expected = [
            Focus::RecentBooks,
            Focus::PathField,
            Focus::Link,
            Focus::SelectionEndpoint,
            Focus::SearchField,
            Focus::SearchHistoryItem,
            Focus::SearchResult,
            Focus::TableOfContentsItem,
            Focus::AnnotationItem,
            Focus::BookmarkNameField,
            Focus::HighlightColor,
            Focus::NoteField,
            Focus::ThemeOption,
            Focus::ConfirmationAction,
            Focus::RecoveryAction,
            Focus::SuspendedView,
        ];

        for (view, expected_focus) in views.into_iter().zip(expected) {
            let app = app_with_view(view);
            assert_eq!(app.focus(), expected_focus);
        }

        let file = tempfile::Builder::new()
            .prefix("reader-focus")
            .suffix(".txt")
            .tempfile()
            .expect("create reader focus fixture");
        let mut reader = App::open(StartupOptions {
            book: Some(file.path().to_path_buf()),
            ..StartupOptions::default()
        })
        .expect("open reader fixture");
        assert!(matches!(reader.view(), View::Reader { .. }));
        assert_eq!(reader.focus(), Focus::ReadingAnchor);
        reader.update(Action::ShowHelp);
        assert_eq!(reader.focus(), Focus::Help);
    }

    #[test]
    fn con_002_003_production_navigation_and_resize_roll_image_generation() -> Result<()> {
        use std::io::Write as _;

        let mut file = tempfile::Builder::new().suffix(".txt").tempfile()?;
        writeln!(file, "first line\nsecond line\nthird line")?;
        let mut app = App::open(StartupOptions {
            book: Some(file.path().to_path_buf()),
            ..StartupOptions::default()
        })?;
        let initial = app
            .reader
            .as_ref()
            .expect("reader")
            .image_workers
            .generation();

        app.set_content_viewport(40, 10);
        let resized = app
            .reader
            .as_ref()
            .expect("reader")
            .image_workers
            .generation();
        assert_ne!(initial, resized, "production resize cancels old image work");

        app.update(Action::DocumentEnd);
        let navigated = app
            .reader
            .as_ref()
            .expect("reader")
            .image_workers
            .generation();
        assert_ne!(
            resized, navigated,
            "production navigation cancels old image work"
        );
        Ok(())
    }

    #[test]
    fn con_005_term_009_app_shutdown_joins_production_image_workers() -> Result<()> {
        use std::io::Write as _;

        let mut file = tempfile::Builder::new().suffix(".txt").tempfile()?;
        writeln!(file, "worker shutdown")?;
        let mut app = App::open(StartupOptions {
            book: Some(file.path().to_path_buf()),
            ..StartupOptions::default()
        })?;

        app.request_worker_shutdown();
        app.join_workers();

        assert_eq!(
            app.reader
                .as_ref()
                .expect("reader")
                .image_workers
                .stats()
                .live_workers,
            0
        );
        Ok(())
    }

    #[test]
    fn img_013_more_than_completion_capacity_never_stays_loading() -> Result<()> {
        use crate::document::{Block, BlockKind, DocumentId, Section};
        use std::fmt::Write as _;

        let directory = tempfile::tempdir()?;
        let book = directory.path().join("many.md");
        std::fs::write(&book, "many images")?;
        let png = {
            let source = image::RgbaImage::from_pixel(2, 2, image::Rgba([12, 34, 56, 255]));
            let mut cursor = std::io::Cursor::new(Vec::new());
            image::DynamicImage::ImageRgba8(source)
                .write_to(&mut cursor, image::ImageFormat::Png)?;
            cursor.into_inner()
        };
        std::fs::write(directory.path().join("plate.png"), &png)?;

        let mut canonical = String::new();
        let mut blocks = Vec::new();
        for index in 0..12 {
            let start = canonical.len();
            write!(canonical, "[image: plate {index}]")?;
            blocks.push(Block::image(
                start..canonical.len(),
                ImageResource::member("plate.png", Some(png.len() as u64)),
            ));
        }
        let document = Document::from_sections(
            DocumentId::new("saturation".to_owned()),
            None,
            canonical,
            vec![Section::new(None, blocks)],
        )
        .map_err(anyhow::Error::msg)?;
        assert!(
            document.sections()[0]
                .blocks()
                .iter()
                .all(|block| block.kind() == BlockKind::Image)
        );
        let provider = ResourceProvider::markdown(&book)?;
        let mut session = ReaderSession::new(document, provider)?;

        let overlays =
            session.prepare_visible_images(80, 100, ImageBackend::TrueColorCells, Rgb(0, 0, 0));
        assert_eq!(overlays.len(), 12);
        std::thread::sleep(std::time::Duration::from_millis(100));
        for _ in 0..100 {
            session.drain_image_work();
            if session
                .images
                .values()
                .all(|state| !matches!(state, ImageState::Loading { .. }))
            {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }

        assert_eq!(session.images.len(), 12);
        assert!(
            session
                .images
                .values()
                .all(|state| !matches!(state, ImageState::Loading { .. }))
        );
        assert!(
            session.image_workers.stats().rejected > 0
                || session.image_workers.stats().dropped_completions > 0,
            "the fixture must cross at least one coordinator capacity boundary"
        );
        Ok(())
    }

    #[test]
    fn img_004_declared_extensions_enable_tga_svg_and_svgz_with_magic_precedence() -> Result<()> {
        use std::io::Write as _;

        let limits = crate::document::ImageLimits::default();
        let tga = {
            let source = image::RgbaImage::from_pixel(2, 2, image::Rgba([90, 80, 70, 255]));
            let mut cursor = std::io::Cursor::new(Vec::new());
            image::DynamicImage::ImageRgba8(source)
                .write_to(&mut cursor, image::ImageFormat::Tga)?;
            cursor.into_inner()
        };
        let decoded = decode_image_bytes(&tga, &limits, declared_image_format("safe/plate.tga"))?;
        assert_eq!((decoded.width(), decoded.height()), (2, 2));

        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" width="2" height="3"><rect width="2" height="3" fill="red"/></svg>"#;
        let decoded = decode_image_bytes(svg, &limits, declared_image_format("plate.svg"))?;
        assert_eq!((decoded.width(), decoded.height()), (2, 3));

        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(svg)?;
        let svgz = encoder.finish()?;
        let decoded = decode_image_bytes(&svgz, &limits, declared_image_format("plate.svgz"))?;
        assert_eq!((decoded.width(), decoded.height()), (2, 3));

        let png = {
            let source = image::RgbaImage::from_pixel(1, 1, image::Rgba([1, 2, 3, 255]));
            let mut cursor = std::io::Cursor::new(Vec::new());
            image::DynamicImage::ImageRgba8(source)
                .write_to(&mut cursor, image::ImageFormat::Png)?;
            cursor.into_inner()
        };
        let decoded = decode_image_bytes(&png, &limits, declared_image_format("wrong.tga"))?;
        assert_eq!(decoded.rgba(), &vec![1, 2, 3, 255]);
        Ok(())
    }
}

#[cfg(test)]
mod toc_tests {
    use super::*;
    use crate::document::{Block, BlockKind, DocumentId, NavigationPoint, Section};

    const EPUB2: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/epub/minimal-epub2.epub"
    );

    fn reader_app() -> Result<App> {
        App::open(StartupOptions {
            book: Some(std::path::PathBuf::from(EPUB2)),
            ..StartupOptions::default()
        })
    }

    fn app_with_document(document: Document) -> Result<App> {
        Ok(App {
            view: View::Reader {
                book: OpenBook {
                    path: PathBuf::from("navigation-test.epub"),
                },
            },
            running: true,
            theme: ThemeName::Paper,
            no_color: false,
            color_mode: ColorMode::TrueColor,
            image_backend_override: None,
            theme_cursor: ThemeName::Paper as usize,
            toc_cursor: 0,
            message: None,
            reader: Some(ReaderSession::new(document, ResourceProvider::None)?),
            content_width: MINIMUM_WIDTH,
            content_height: MINIMUM_HEIGHT,
        })
    }

    #[test]
    fn section_navigation_preserves_declared_selection_across_duplicates_and_resize() -> Result<()>
    {
        let base = Document::from_sections(
            DocumentId::new("declared-navigation".to_owned()),
            None,
            "one\ntwo\nthree".to_owned(),
            vec![
                Section::new(None, vec![Block::new(BlockKind::Paragraph, 0..4)]),
                Section::new(None, vec![Block::new(BlockKind::Paragraph, 4..8)]),
                Section::new(None, vec![Block::new(BlockKind::Paragraph, 8..13)]),
            ],
        )
        .expect("document");
        let first = base.position(0, 0, 0).expect("first");
        let middle = base.position(1, 0, 0).expect("middle");
        let last = base.position(2, 0, 0).expect("last");
        let document = base.with_navigation(vec![
            NavigationPoint::new("Last", last),
            NavigationPoint::new("First", first),
            NavigationPoint::new("First duplicate", first),
            NavigationPoint::new("Middle", middle),
        ]);
        let mut app = app_with_document(document)?;

        app.update(Action::ShowToc);
        assert_eq!(app.toc_cursor(), 1);
        app.update(Action::NextLine);
        app.update(Action::Confirm);
        assert_eq!(app.reader().expect("reader").navigation_index, Some(2));
        assert_eq!(app.reader().expect("reader").anchor(), first);

        app.set_content_viewport(MINIMUM_WIDTH + 7, MINIMUM_HEIGHT + 3);
        assert_eq!(app.reader().expect("reader").navigation_index, Some(2));
        assert_eq!(app.reader().expect("reader").anchor(), first);

        app.update(Action::SectionEnd);
        assert_eq!(app.reader().expect("reader").navigation_index, Some(3));
        assert_eq!(app.reader().expect("reader").anchor(), middle);
        app.update(Action::SectionStart);
        assert_eq!(app.reader().expect("reader").navigation_index, Some(2));
        assert_eq!(app.reader().expect("reader").anchor(), first);

        app.update(Action::ShowToc);
        app.update(Action::PreviousLine);
        app.update(Action::PreviousLine);
        app.update(Action::Confirm);
        assert_eq!(app.reader().expect("reader").navigation_index, Some(0));
        assert_eq!(app.reader().expect("reader").anchor(), last);
        app.update(Action::SectionEnd);
        assert_eq!(app.reader().expect("reader").navigation_index, Some(1));
        assert_eq!(app.reader().expect("reader").anchor(), first);
        app.update(Action::SectionStart);
        assert_eq!(app.reader().expect("reader").navigation_index, Some(0));
        assert_eq!(app.reader().expect("reader").anchor(), last);

        app.request_worker_shutdown();
        app.join_workers();
        Ok(())
    }

    #[test]
    fn nav_009_toc_opens_on_the_current_section_and_jumps_by_confirm() -> Result<()> {
        let mut app = reader_app()?;

        // Land inside the final section before opening the contents.
        app.update(Action::DocumentEnd);
        let second_start = app
            .reader()
            .expect("book")
            .anchor
            .absolute_byte(app.reader().expect("book").document());
        assert!(second_start > 0);

        app.update(Action::ShowToc);
        assert!(matches!(app.view(), View::TableOfContents { .. }));
        assert_eq!(app.focus(), Focus::TableOfContentsItem);
        assert_eq!(
            app.toc_cursor(),
            1,
            "the overlay opens on the current section"
        );

        app.update(Action::PreviousLine);
        app.update(Action::Confirm);
        assert!(
            matches!(app.view(), View::Reader { .. }),
            "confirm returns to reading"
        );
        let anchor = app.reader().expect("book").anchor;
        assert_eq!(anchor.section(), 0, "the first section jump lands");
        assert_eq!(
            anchor.absolute_byte(app.reader().expect("book").document()),
            0
        );

        let message = app.message().expect("confirmation message").text();
        assert!(message.contains("Jumped:"), "{message}");
        Ok(())
    }

    #[test]
    fn nav_009_toc_back_and_help_round_trip_preserve_state() -> Result<()> {
        let mut app = reader_app()?;
        app.update(Action::ShowToc);
        app.update(Action::NextLine);
        app.update(Action::Back);
        assert!(matches!(app.view(), View::Reader { .. }));

        app.update(Action::ShowToc);
        app.update(Action::ShowHelp);
        assert_eq!(app.focus(), Focus::Help);
        app.update(Action::Back);
        assert!(
            matches!(app.view(), View::TableOfContents { .. }),
            "help returns into the contents list"
        );
        Ok(())
    }

    #[test]
    fn nav_009_show_toc_without_a_book_is_inert() -> Result<()> {
        let mut app = App::open(StartupOptions::default())?;
        app.update(Action::ShowToc);
        assert_eq!(app.view(), &View::RecentBooks);
        Ok(())
    }
}
