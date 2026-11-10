//! Drawing: Direct2D into a DirectComposition surface, so the window has no
//! redirection bitmap and is transparent wherever the island isn't drawn.
//! Text is DirectWrite; the cover is decoded with WIC.

use island_core::glance;
use island_core::lyrics::Line;
use island_core::media::{self as layout, Button, SongText, TextAlign, fmt_time};
use island_core::menu;
use island_core::pages::{self, FocusButton, Page};
use island_core::shape::{Outline, Point, Rect, Segment};
use windows::Win32::Foundation::{E_FAIL, HMODULE, HWND, POINT, RECT};
use windows::Win32::Graphics::Direct2D::Common::{
    D2D_RECT_F, D2D_SIZE_F, D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_FIGURE_BEGIN_FILLED,
    D2D1_FIGURE_END_CLOSED, D2D1_GRADIENT_STOP, D2D1_PIXEL_FORMAT,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1_ANTIALIAS_MODE_ALIASED, D2D1_ARC_SEGMENT, D2D1_ARC_SIZE_SMALL,
    D2D1_BITMAP_BRUSH_PROPERTIES1, D2D1_BITMAP_OPTIONS_CANNOT_DRAW, D2D1_BITMAP_OPTIONS_TARGET,
    D2D1_BITMAP_PROPERTIES1, D2D1_DEVICE_CONTEXT_OPTIONS_NONE, D2D1_DRAW_TEXT_OPTIONS_CLIP,
    D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT, D2D1_ELLIPSE, D2D1_EXTEND_MODE_CLAMP,
    D2D1_FACTORY_TYPE_SINGLE_THREADED, D2D1_GAMMA_2_2, D2D1_INTERPOLATION_MODE_LINEAR,
    D2D1_LINEAR_GRADIENT_BRUSH_PROPERTIES, D2D1_ROUNDED_RECT, D2D1_SWEEP_DIRECTION_CLOCKWISE,
    D2D1_SWEEP_DIRECTION_COUNTER_CLOCKWISE, D2D1CreateFactory, ID2D1BitmapBrush1, ID2D1Brush,
    ID2D1DeviceContext, ID2D1Factory1, ID2D1Image, ID2D1PathGeometry, ID2D1RenderTarget,
    ID2D1SolidColorBrush,
};
use windows::Win32::Graphics::Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_WARP};
use windows::Win32::Graphics::Direct3D11::{
    D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_SDK_VERSION, D3D11CreateDevice, ID3D11Device,
};
use windows::Win32::Graphics::DirectComposition::{
    DCompositionCreateDevice, IDCompositionDevice, IDCompositionSurface, IDCompositionTarget,
    IDCompositionVisual,
};
use windows::Win32::Graphics::DirectWrite::{
    DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL,
    DWRITE_FONT_WEIGHT, DWRITE_FONT_WEIGHT_NORMAL, DWRITE_FONT_WEIGHT_SEMI_BOLD,
    DWRITE_MEASURING_MODE_NATURAL, DWRITE_PARAGRAPH_ALIGNMENT_CENTER,
    DWRITE_PARAGRAPH_ALIGNMENT_NEAR, DWRITE_TEXT_ALIGNMENT, DWRITE_TEXT_ALIGNMENT_CENTER,
    DWRITE_TEXT_ALIGNMENT_LEADING, DWRITE_TEXT_ALIGNMENT_TRAILING, DWRITE_TEXT_METRICS,
    DWRITE_TEXT_RANGE, DWRITE_TRIMMING, DWRITE_TRIMMING_GRANULARITY_CHARACTER,
    DWRITE_WORD_WRAPPING_NO_WRAP, DWRITE_WORD_WRAPPING_WRAP, DWriteCreateFactory, IDWriteFactory,
    IDWriteFontCollection, IDWriteTextFormat, IDWriteTextLayout,
};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_ALPHA_MODE_PREMULTIPLIED, DXGI_FORMAT_B8G8R8A8_UNORM,
};
use windows::Win32::Graphics::Dxgi::{IDXGIAdapter, IDXGIDevice, IDXGISurface};
use windows::Win32::Graphics::Imaging::{
    CLSID_WICImagingFactory, GUID_WICPixelFormat32bppBGRA, GUID_WICPixelFormat32bppPBGRA,
    IWICBitmapFrameDecode, IWICImagingFactory, WICBitmapDitherTypeNone,
    WICBitmapInterpolationModeFant, WICBitmapPaletteTypeMedianCut, WICDecodeMetadataCacheOnDemand,
};
use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance};
use windows::core::{BOOL, Error, Interface, PCWSTR, Result, w};
use windows_numerics::{Matrix3x2, Vector2};

use crate::bars::Bars;
use crate::media::Track;

/// What to draw in a frame.
pub struct Scene<'a> {
    /// The window, in px.
    pub width: u32,
    pub height: u32,
    /// The body (ears included), in px.
    pub body: Rect,
    pub outline: &'a Outline,
    pub scale: f32,
    /// From the body's left edge to its content (the ear), in px.
    pub inset: f32,
    /// The body without ears, in DIPs.
    pub size: (f32, f32),
    /// How visible the closed, glance and open contents are (0 to 1).
    pub alpha: [f32; 3],
    pub track: Option<&'a Track>,
    pub position_ms: u64,
    pub empty: &'a str,
    /// What the song's line says (closed and peek).
    pub song: SongText,
    /// With synced lyrics: where the song is in them.
    pub lyrics: Option<LyricsView<'a>>,
    /// How the open island shows them beside the player (off without them).
    pub open: layout::Open,
    /// The glance shows the line being sung instead of the song.
    pub glance: bool,
    /// The open island's page, and the tabs' right end: the time of day,
    /// or the focus timer while it runs.
    pub page: Page,
    pub tabs_clock: &'a str,
    pub tabs_timer: bool,
    pub focus: FocusView<'a>,
    /// A glance that tells something (the volume, the focus timer's end)
    /// instead of the song.
    pub notice: Option<Notice<'a>>,
    /// With nothing playing, the closed island shows the focus timer.
    pub closed_timer: Option<&'a str>,
}

/// What a glance tells: an icon in its color, a title, a dimmer detail
/// and, for a level (the volume), a bar.
#[derive(Clone, Copy)]
pub struct Notice<'a> {
    pub icon: &'a str,
    pub tint: [f32; 3],
    pub title: &'a str,
    pub detail: &'a str,
    pub level: Option<f32>,
}

/// The focus page, ready to draw.
#[derive(Clone, Copy)]
pub struct FocusView<'a> {
    /// "Focus · round 2 of 4".
    pub label: &'a str,
    pub time: &'a str,
    pub next: &'a str,
    pub progress: f32,
    pub rounds: u32,
    pub every: u32,
    pub running: bool,
    /// A break (drawn green; focus is warm).
    pub resting: bool,
}

impl FocusView<'_> {
    pub fn tint(&self) -> [f32; 3] {
        if self.resting { BREAK_TINT } else { FOCUS_TINT }
    }
}

/// The focus timer's colors.
pub const FOCUS_TINT: [f32; 3] = [1.0, 0.62, 0.32];
pub const BREAK_TINT: [f32; 3] = [0.42, 0.86, 0.6];
/// Plain white, for glances with no color of their own.
pub const WHITE: [f32; 3] = [1.0; 3];

#[derive(Clone, Copy)]
pub struct LyricsView<'a> {
    pub lines: &'a [Line],
    /// The line being sung; `None` before the first.
    pub index: Option<usize>,
    /// How much of the scroll to it is left (see [`layout::lyrics_scroll`]).
    pub scroll: f32,
}

impl LyricsView<'_> {
    /// The words being sung; empty in a pause.
    pub fn current(&self) -> &str {
        self.index
            .and_then(|i| self.lines.get(i))
            .map_or("", |l| l.text.as_str())
    }
}

/// The open and closed layouts are drawn at their full size, centered in
/// the body, so they don't reflow while it grows.
const CLOSED_W: f32 = 180.0;
/// The note that stands in for a missing cover is laid out in a box this
/// big (DIPs) and scaled to the cover's size.
const NOTE_BOX: f32 = 44.0;

struct Art {
    /// The `Arc` it was decoded from, by address.
    key: usize,
    brush: ID2D1BitmapBrush1,
    /// The square it fills, in px, and the bitmap's own size (covering it).
    side: f32,
    size: (f32, f32),
    /// Its color, for the bars and the glow at the bottom.
    accent: [f32; 3],
}

struct Text {
    dw: IDWriteFactory,
    /// The lyrics' lines: they wrap, and all have one size, so a line
    /// keeps its height as it comes into focus.
    lines: IDWriteTextFormat,
    title: IDWriteTextFormat,
    artist: IDWriteTextFormat,
    time: IDWriteTextFormat,
    time_end: IDWriteTextFormat,
    empty: IDWriteTextFormat,
    icon: IDWriteTextFormat,
    peek: IDWriteTextFormat,
    closed: IDWriteTextFormat,
    note: IDWriteTextFormat,
    menu: IDWriteTextFormat,
    menu_head: IDWriteTextFormat,
    tab: IDWriteTextFormat,
    clock: IDWriteTextFormat,
    big: IDWriteTextFormat,
    next: IDWriteTextFormat,
}

/// One entry of the menu, as drawn.
pub struct MenuRow<'a> {
    pub kind: menu::Kind,
    pub label: &'a str,
    pub on: bool,
    pub enabled: bool,
}

/// What the menu shows in a frame.
pub struct MenuView<'a> {
    /// The window, in px.
    pub width: u32,
    pub height: u32,
    pub scale: f32,
    pub title: &'a str,
    pub version: &'a str,
    pub rows: &'a [MenuRow<'a>],
    pub hover: Option<usize>,
}

/// The menu window's own composition target; drawn with the island's
/// device, so the menu costs no second one.
pub struct MenuTarget {
    _target: IDCompositionTarget,
    visual: IDCompositionVisual,
    surface: Option<(IDCompositionSurface, u32, u32)>,
}

pub struct Renderer {
    factory: ID2D1Factory1,
    dc: ID2D1DeviceContext,
    dcomp: IDCompositionDevice,
    // Kept alive: dropping the target detaches the visual from the window.
    _target: IDCompositionTarget,
    visual: IDCompositionVisual,
    surface: Option<(IDCompositionSurface, u32, u32)>,
    fill: ID2D1SolidColorBrush,
    ink: ID2D1SolidColorBrush,
    /// The artist next to the title.
    dim: ID2D1SolidColorBrush,
    text: Text,
    wic: IWICImagingFactory,
    art: Option<Art>,
    /// The cover's color in this frame; `None` without a cover.
    accent: Option<[f32; 3]>,
    bars: Bars,
    /// Where this frame wants the bars (DIPs), how visible, and if playing.
    bars_at: Option<(Rect, f32, bool)>,
    /// The view being drawn: its scale's center x, the scale and how far
    /// above its place it is (see [`layout::entrance`]).
    view: (f32, f32, f32),
    /// The body's bounds in the last frame (px): the next frame redraws
    /// only those and its own, and the rest of the surface is kept.
    drawn: Option<RECT>,
}

fn d3d_device() -> Result<ID3D11Device> {
    // WARP (software) first: the island is small, and the GPU driver alone
    // costs ~50 MB of private memory (7 MB with WARP, measured on an Intel
    // UHD 730). The GPU only if WARP fails.
    for driver in [D3D_DRIVER_TYPE_WARP, D3D_DRIVER_TYPE_HARDWARE] {
        let mut device = None;
        let made = unsafe {
            D3D11CreateDevice(
                None::<&IDXGIAdapter>,
                driver,
                HMODULE::default(),
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                None,
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                None,
            )
        };
        if let (Ok(()), Some(device)) = (made, device) {
            return Ok(device);
        }
    }
    Err(Error::from(E_FAIL))
}

fn v(p: Point) -> Vector2 {
    Vector2 { X: p.x, Y: p.y }
}

/// The menu's panel.
const MENU_PANEL: D2D1_COLOR_F = D2D1_COLOR_F {
    r: 0.17,
    g: 0.17,
    b: 0.18,
    a: 0.97,
};

fn white(a: f32) -> D2D1_COLOR_F {
    D2D1_COLOR_F {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a,
    }
}

fn rf(r: Rect) -> D2D_RECT_F {
    D2D_RECT_F {
        left: r.x,
        top: r.y,
        right: r.x + r.width,
        bottom: r.y + r.height,
    }
}

fn shift(r: Rect, dx: f32) -> Rect {
    Rect { x: r.x + dx, ..r }
}

fn utf16(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

/// Scale `k`, then move by (`x`, `y`).
fn affine(k: f32, x: f32, y: f32) -> Matrix3x2 {
    Matrix3x2 {
        M11: k,
        M12: 0.0,
        M21: 0.0,
        M22: k,
        M31: x,
        M32: y,
    }
}

fn has_family(dw: &IDWriteFactory, name: PCWSTR) -> bool {
    let mut fonts: Option<IDWriteFontCollection> = None;
    let (mut index, mut exists) = (0, BOOL(0));
    unsafe {
        dw.GetSystemFontCollection(&mut fonts, false).is_ok()
            && fonts.is_some_and(|f| {
                f.FindFamilyName(name, &mut index, &mut exists).is_ok() && exists.as_bool()
            })
    }
}

impl Text {
    fn new() -> Result<Self> {
        unsafe {
            let dw: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;
            let format = |family: PCWSTR,
                          weight: DWRITE_FONT_WEIGHT,
                          size: f32,
                          align: DWRITE_TEXT_ALIGNMENT|
             -> Result<IDWriteTextFormat> {
                let f = dw.CreateTextFormat(
                    family,
                    None,
                    weight,
                    DWRITE_FONT_STYLE_NORMAL,
                    DWRITE_FONT_STRETCH_NORMAL,
                    size,
                    w!(""),
                )?;
                f.SetWordWrapping(DWRITE_WORD_WRAPPING_NO_WRAP)?;
                f.SetTextAlignment(align)?;
                f.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER)?;
                let trimming = DWRITE_TRIMMING {
                    granularity: DWRITE_TRIMMING_GRANULARITY_CHARACTER,
                    delimiter: 0,
                    delimiterCount: 0,
                };
                f.SetTrimming(&trimming, &dw.CreateEllipsisTrimmingSign(&f)?)?;
                Ok(f)
            };
            let ui = w!("Segoe UI");
            // Windows 11 has Fluent icons; Windows 10 the MDL2 ones, with the
            // same code points for these.
            let icons = if has_family(&dw, w!("Segoe Fluent Icons")) {
                w!("Segoe Fluent Icons")
            } else {
                w!("Segoe MDL2 Assets")
            };
            let (n, b) = (DWRITE_FONT_WEIGHT_NORMAL, DWRITE_FONT_WEIGHT_SEMI_BOLD);
            let lines = format(ui, b, 16.0, DWRITE_TEXT_ALIGNMENT_LEADING)?;
            lines.SetWordWrapping(DWRITE_WORD_WRAPPING_WRAP)?;
            lines.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_NEAR)?;
            Ok(Self {
                lines,
                title: format(ui, b, 17.0, DWRITE_TEXT_ALIGNMENT_LEADING)?,
                artist: format(ui, n, 13.0, DWRITE_TEXT_ALIGNMENT_LEADING)?,
                time: format(ui, n, 11.0, DWRITE_TEXT_ALIGNMENT_LEADING)?,
                time_end: format(ui, n, 11.0, DWRITE_TEXT_ALIGNMENT_TRAILING)?,
                empty: format(ui, n, 14.0, DWRITE_TEXT_ALIGNMENT_CENTER)?,
                icon: format(icons, n, 18.0, DWRITE_TEXT_ALIGNMENT_CENTER)?,
                peek: format(ui, b, 13.0, DWRITE_TEXT_ALIGNMENT_LEADING)?,
                closed: format(ui, b, 12.0, DWRITE_TEXT_ALIGNMENT_LEADING)?,
                note: format(icons, n, 22.0, DWRITE_TEXT_ALIGNMENT_CENTER)?,
                menu: format(ui, n, 12.0, DWRITE_TEXT_ALIGNMENT_LEADING)?,
                menu_head: format(ui, b, 12.0, DWRITE_TEXT_ALIGNMENT_LEADING)?,
                tab: format(icons, n, 15.0, DWRITE_TEXT_ALIGNMENT_CENTER)?,
                clock: format(ui, b, 12.0, DWRITE_TEXT_ALIGNMENT_TRAILING)?,
                big: format(ui, b, 42.0, DWRITE_TEXT_ALIGNMENT_LEADING)?,
                next: format(ui, n, 12.0, DWRITE_TEXT_ALIGNMENT_TRAILING)?,
                dw: dw.clone(),
            })
        }
    }
}

impl Renderer {
    pub fn new(hwnd: HWND) -> Result<Self> {
        let dxgi: IDXGIDevice = d3d_device()?.cast()?;
        unsafe {
            let factory: ID2D1Factory1 =
                D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
            let dc = factory
                .CreateDevice(&dxgi)?
                .CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)?;
            let dcomp: IDCompositionDevice = DCompositionCreateDevice(&dxgi)?;
            let target = dcomp.CreateTargetForHwnd(hwnd, true)?;
            let visual = dcomp.CreateVisual()?;
            target.SetRoot(&visual)?;
            let black = D2D1_COLOR_F {
                a: 1.0,
                ..Default::default()
            };
            let fill = dc.CreateSolidColorBrush(&black, None)?;
            let ink = dc.CreateSolidColorBrush(&white(1.0), None)?;
            let dim = dc.CreateSolidColorBrush(&white(0.55), None)?;
            let bars = Bars::new(&dcomp, &visual)?;
            Ok(Self {
                bars,
                bars_at: None,
                view: (0.0, 1.0, 0.0),
                drawn: None,
                factory,
                dc,
                dcomp,
                _target: target,
                visual,
                surface: None,
                fill,
                ink,
                dim,
                text: Text::new()?,
                wic: CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)?,
                art: None,
                accent: None,
            })
        }
    }

    /// Where the song's line sits, closed and in the peek.
    pub fn align_song(&self, align: TextAlign) -> Result<()> {
        let a = match align {
            TextAlign::Left => DWRITE_TEXT_ALIGNMENT_LEADING,
            TextAlign::Center => DWRITE_TEXT_ALIGNMENT_CENTER,
        };
        unsafe {
            self.text.closed.SetTextAlignment(a)?;
            self.text.peek.SetTextAlignment(a)
        }
    }

    fn surface(&mut self, width: u32, height: u32) -> Result<IDCompositionSurface> {
        if let Some((s, w, h)) = &self.surface
            && (*w, *h) == (width, height)
        {
            return Ok(s.clone());
        }
        let s = unsafe {
            let s = self.dcomp.CreateSurface(
                width,
                height,
                DXGI_FORMAT_B8G8R8A8_UNORM,
                DXGI_ALPHA_MODE_PREMULTIPLIED,
            )?;
            self.visual.SetContent(&s)?;
            s
        };
        self.surface = Some((s.clone(), width, height));
        // A new surface starts empty: the first frame draws all of it.
        self.drawn = None;
        Ok(s)
    }

    fn geometry(&self, o: &Outline) -> Result<ID2D1PathGeometry> {
        unsafe {
            let geo: ID2D1PathGeometry = self.factory.CreatePathGeometry()?.cast()?;
            let sink = geo.Open()?;
            sink.BeginFigure(v(o.start), D2D1_FIGURE_BEGIN_FILLED);
            for s in &o.segments {
                match *s {
                    Segment::Line(p) => sink.AddLine(v(p)),
                    Segment::Arc {
                        to,
                        radius,
                        clockwise,
                    } => sink.AddArc(&D2D1_ARC_SEGMENT {
                        point: v(to),
                        size: D2D_SIZE_F {
                            width: radius,
                            height: radius,
                        },
                        rotationAngle: 0.0,
                        sweepDirection: if clockwise {
                            D2D1_SWEEP_DIRECTION_CLOCKWISE
                        } else {
                            D2D1_SWEEP_DIRECTION_COUNTER_CLOCKWISE
                        },
                        arcSize: D2D1_ARC_SIZE_SMALL,
                    }),
                }
            }
            sink.EndFigure(D2D1_FIGURE_END_CLOSED);
            sink.Close()?;
            Ok(geo)
        }
    }

    /// The cover as a brush, decoded at `side` px (the open size on this
    /// screen) once per song; cropped to a square, centered.
    fn art(&mut self, bytes: &std::sync::Arc<Vec<u8>>, side: f32) -> Option<&Art> {
        let key = std::sync::Arc::as_ptr(bytes) as usize;
        let fresh = self
            .art
            .as_ref()
            .is_some_and(|a| a.key == key && a.side == side);
        if !fresh {
            self.art = self
                .decode(bytes, side)
                .ok()
                .map(|(brush, size, accent)| Art {
                    key,
                    brush,
                    side,
                    size,
                    accent,
                });
        }
        self.art.as_ref()
    }

    /// The cover's color, from an 8 x 8 thumbnail of it.
    fn accent_of(&self, frame: &IWICBitmapFrameDecode) -> Result<[f32; 3]> {
        unsafe {
            let small = self.wic.CreateBitmapScaler()?;
            small.Initialize(frame, 8, 8, WICBitmapInterpolationModeFant)?;
            let bgra = self.wic.CreateFormatConverter()?;
            bgra.Initialize(
                &small,
                &GUID_WICPixelFormat32bppBGRA,
                WICBitmapDitherTypeNone,
                None,
                0.0,
                WICBitmapPaletteTypeMedianCut,
            )?;
            let mut buf = [0u8; 8 * 8 * 4];
            bgra.CopyPixels(std::ptr::null(), 8 * 4, &mut buf)?;
            let px: Vec<[u8; 3]> = buf.chunks(4).map(|p| [p[2], p[1], p[0]]).collect();
            Ok(layout::accent(&px))
        }
    }

    fn decode(&self, bytes: &[u8], side: f32) -> Result<(ID2D1BitmapBrush1, (f32, f32), [f32; 3])> {
        unsafe {
            let stream = self.wic.CreateStream()?;
            stream.InitializeFromMemory(bytes)?;
            let decoder = self.wic.CreateDecoderFromStream(
                &stream,
                std::ptr::null(),
                WICDecodeMetadataCacheOnDemand,
            )?;
            let frame = decoder.GetFrame(0)?;
            let accent = self.accent_of(&frame).unwrap_or([0.85; 3]);
            let (mut fw, mut fh) = (0, 0);
            frame.GetSize(&mut fw, &mut fh)?;
            let short = fw.min(fh).max(1) as f32;
            let (w, h) = (
                (side * fw as f32 / short).ceil().max(1.0),
                (side * fh as f32 / short).ceil().max(1.0),
            );
            let scaler = self.wic.CreateBitmapScaler()?;
            scaler.Initialize(&frame, w as u32, h as u32, WICBitmapInterpolationModeFant)?;
            let pbgra = self.wic.CreateFormatConverter()?;
            pbgra.Initialize(
                &scaler,
                &GUID_WICPixelFormat32bppPBGRA,
                WICBitmapDitherTypeNone,
                None,
                0.0,
                WICBitmapPaletteTypeMedianCut,
            )?;
            let bitmap = self.dc.CreateBitmapFromWicBitmap(&pbgra, None)?;
            let brush = self.dc.CreateBitmapBrush(
                &bitmap,
                Some(&D2D1_BITMAP_BRUSH_PROPERTIES1 {
                    extendModeX: D2D1_EXTEND_MODE_CLAMP,
                    extendModeY: D2D1_EXTEND_MODE_CLAMP,
                    interpolationMode: D2D1_INTERPOLATION_MODE_LINEAR,
                }),
                None,
            )?;
            Ok((brush, (w, h), accent))
        }
    }

    /// Fills `r` (DIPs, in the content space) with the cover, or a dim
    /// square when there is none.
    fn draw_art(&mut self, track: &Track, r: Rect, radius: f32, alpha: f32, scale: f32) {
        let rounded = D2D1_ROUNDED_RECT {
            rect: rf(r),
            radiusX: radius,
            radiusY: radius,
        };
        let side = (layout::ART.width * scale).ceil();
        let dc = self.dc.clone();
        let art = track.art.as_ref().and_then(|b| self.art(b, side));
        unsafe {
            match art {
                Some(a) => {
                    // Bitmap px to this rect: scale it down, center the crop.
                    let k = r.width / a.side;
                    let (dx, dy) = ((a.size.0 - a.side) / 2.0, (a.size.1 - a.side) / 2.0);
                    a.brush.SetTransform(&affine(k, r.x - dx * k, r.y - dy * k));
                    a.brush.SetOpacity(alpha);
                    dc.FillRoundedRectangle(&rounded, &a.brush);
                }
                // No cover: a note on a dim square, so it never looks empty.
                None => {
                    self.ink.SetColor(&white(0.14 * alpha));
                    dc.FillRoundedRectangle(&rounded, &self.ink);
                    let mut m = Matrix3x2::default();
                    dc.GetTransform(&mut m);
                    let k = r.width / NOTE_BOX;
                    dc.SetTransform(&(affine(k, r.x, r.y) * m));
                    let box_ = Rect {
                        x: 0.0,
                        y: 0.0,
                        width: NOTE_BOX,
                        height: NOTE_BOX,
                    };
                    self.text("\u{E8D6}", &self.text.note, box_, 0.6 * alpha);
                    dc.SetTransform(&m);
                }
            }
        }
    }

    fn text(&self, s: &str, format: &IDWriteTextFormat, r: Rect, alpha: f32) {
        unsafe {
            self.ink.SetColor(&white(alpha));
            self.dc.DrawText(
                &utf16(s),
                format,
                &rf(r),
                &self.ink,
                // Emoji in color; nothing drawn past the box.
                D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT | D2D1_DRAW_TEXT_OPTIONS_CLIP,
                DWRITE_MEASURING_MODE_NATURAL,
            );
        }
    }

    /// The song's line: the dim pieces (the artist) at a lower opacity.
    fn song(&self, spans: &[(String, bool)], format: &IDWriteTextFormat, r: Rect, alpha: f32) {
        let text: Vec<u16> = spans.iter().flat_map(|s| s.0.encode_utf16()).collect();
        unsafe {
            let Ok(layout) = self
                .text
                .dw
                .CreateTextLayout(&text, format, r.width, r.height)
            else {
                return;
            };
            self.ink.SetColor(&white(0.95 * alpha));
            self.dim.SetColor(&white(0.55 * alpha));
            let mut at = 0u32;
            for (s, dim) in spans {
                let n = s.encode_utf16().count() as u32;
                if *dim {
                    let _ = layout.SetDrawingEffect(
                        &self.dim,
                        DWRITE_TEXT_RANGE {
                            startPosition: at,
                            length: n,
                        },
                    );
                }
                at += n;
            }
            self.dc.DrawTextLayout(
                v(Point { x: r.x, y: r.y }),
                &layout,
                &self.ink,
                D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT | D2D1_DRAW_TEXT_OPTIONS_CLIP,
            );
        }
    }

    fn rounded(&self, r: Rect, radius: f32, alpha: f32) {
        self.tinted_rounded(r, radius, [1.0; 3], alpha);
    }

    fn tinted_rounded(&self, r: Rect, radius: f32, [r_, g, b]: [f32; 3], a: f32) {
        unsafe {
            self.ink.SetColor(&D2D1_COLOR_F { r: r_, g, b, a });
            self.dc.FillRoundedRectangle(
                &D2D1_ROUNDED_RECT {
                    rect: rf(r),
                    radiusX: radius,
                    radiusY: radius,
                },
                &self.ink,
            );
        }
    }

    fn tinted(&self, s: &str, format: &IDWriteTextFormat, at: Rect, [r, g, b]: [f32; 3], a: f32) {
        unsafe {
            self.ink.SetColor(&D2D1_COLOR_F { r, g, b, a });
            self.dc.DrawText(
                &utf16(s),
                format,
                &rf(at),
                &self.ink,
                D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT | D2D1_DRAW_TEXT_OPTIONS_CLIP,
                DWRITE_MEASURING_MODE_NATURAL,
            );
        }
    }

    fn draw_open(&mut self, s: &Scene, dx: f32, alpha: f32) {
        self.draw_tabs(s, dx, alpha);
        // The page under the tabs, laid out from its own top.
        let mut m = Matrix3x2::default();
        unsafe {
            self.dc.GetTransform(&mut m);
            self.dc
                .SetTransform(&(Matrix3x2::translation(0.0, pages::PAGE_Y) * m));
        }
        match s.page {
            Page::Home => self.draw_home(s, dx, alpha),
            Page::Focus => self.draw_focus(&s.focus, s.open.width, dx, alpha),
        }
        unsafe {
            self.dc.SetTransform(&m);
        }
    }

    /// One icon per page, the one shown on a soft pill; the time of day
    /// (or the focus timer, in its color) at the right.
    fn draw_tabs(&self, s: &Scene, dx: f32, alpha: f32) {
        for (i, page) in Page::ALL.into_iter().enumerate() {
            let r = shift(pages::tab(i), dx);
            let on = page == s.page;
            if on {
                self.rounded(r, r.height / 2.0, 0.14 * alpha);
            }
            let glyph = match page {
                Page::Home => "\u{EC4F}",
                Page::Focus => "\u{E916}",
            };
            let a = if on { 0.95 } else { 0.5 };
            self.text(glyph, &self.text.tab, r, a * alpha);
        }
        let clock = shift(pages::clock(s.open.width), dx);
        if s.tabs_timer {
            self.tinted(s.tabs_clock, &self.text.clock, clock, s.focus.tint(), alpha);
        } else {
            self.text(s.tabs_clock, &self.text.clock, clock, 0.6 * alpha);
        }
    }

    /// The focus page: the phase, the time left big, a bar in the phase's
    /// color, a dot per round, and reset, play / pause and skip.
    fn draw_focus(&self, f: &FocusView, width: f32, dx: f32, alpha: f32) {
        let p = pages::focus_page(width);
        self.text(f.label, &self.text.artist, shift(p.label, dx), 0.6 * alpha);
        self.text(f.time, &self.text.big, shift(p.time, dx), 0.95 * alpha);
        self.text(f.next, &self.text.next, shift(p.next, dx), 0.5 * alpha);
        let bar = shift(p.bar, dx);
        self.rounded(bar, 2.0, 0.16 * alpha);
        if f.progress > 0.0 {
            let done = Rect {
                width: bar.width * f.progress.clamp(0.0, 1.0),
                ..bar
            };
            self.tinted_rounded(done, 2.0, f.tint(), 0.95 * alpha);
        }
        for i in 0..f.every {
            let d = Rect {
                x: dx + p.dots.x + i as f32 * (pages::DOT + pages::DOT_GAP),
                width: pages::DOT,
                ..p.dots
            };
            if i < f.rounds {
                self.tinted_rounded(d, pages::DOT / 2.0, FOCUS_TINT, 0.9 * alpha);
            } else {
                self.rounded(d, pages::DOT / 2.0, 0.2 * alpha);
            }
        }
        let play = if f.running { "\u{E769}" } else { "\u{E768}" };
        for (b, glyph) in [
            (FocusButton::Reset, "\u{E72C}"),
            (FocusButton::PlayPause, play),
            (FocusButton::Skip, "\u{E893}"),
        ] {
            let r = shift(pages::focus_button(b, width), dx);
            self.text(glyph, &self.text.icon, r, 0.95 * alpha);
        }
    }

    fn draw_home(&mut self, s: &Scene, dx: f32, alpha: f32) {
        let Some(t) = s.track else {
            let all = Rect {
                x: dx,
                y: 0.0,
                width: s.open.width,
                height: pages::PAGE_HEIGHT,
            };
            self.text(s.empty, &self.text.empty, all, 0.6 * alpha);
            return;
        };
        let p = layout::player(s.open);
        self.draw_art(t, shift(layout::ART, dx), 10.0, alpha, s.scale);
        self.text(&t.title, &self.text.title, shift(p.title, dx), alpha);
        let artist = if t.artist.is_empty() {
            &t.album
        } else {
            &t.artist
        };
        self.text(artist, &self.text.artist, shift(p.artist, dx), 0.6 * alpha);
        let bar = shift(p.bar, dx);
        self.rounded(bar, 2.0, 0.2 * alpha);
        if t.duration_ms > 0 {
            let done = (s.position_ms as f32 / t.duration_ms as f32).clamp(0.0, 1.0);
            let filled = Rect {
                width: bar.width * done,
                ..bar
            };
            self.rounded(filled, 2.0, 0.9 * alpha);
            let times = shift(p.times, dx);
            self.text(
                &fmt_time(s.position_ms),
                &self.text.time,
                times,
                0.5 * alpha,
            );
            self.text(
                &fmt_time(t.duration_ms),
                &self.text.time_end,
                times,
                0.5 * alpha,
            );
        }
        for (b, glyph, on) in [
            (Button::Prev, "\u{E892}", t.can_prev),
            (
                Button::PlayPause,
                if t.playing { "\u{E769}" } else { "\u{E768}" },
                t.can_toggle,
            ),
            (Button::Next, "\u{E893}", t.can_next),
        ] {
            let a = if on { 0.95 } else { 0.3 } * alpha;
            let r = layout::button(b, s.open);
            self.text(glyph, &self.text.icon, shift(r, dx), a);
        }
        if let (Some(l), Some(col)) = (s.lyrics, s.open.lyrics_rect()) {
            self.draw_lyrics(&l, shift(col, dx), alpha);
        }
    }

    /// The column of lyrics: the line being sung in full light, the ones
    /// around it dimmer the farther they are, all gliding up on a change.
    fn draw_lyrics(&self, l: &LyricsView, col: Rect, alpha: f32) {
        // A few lines before and after are enough to fill the column.
        let (first, current) = match l.index {
            Some(i) => (i.saturating_sub(3), i),
            None => (0, 0),
        };
        let mut texts: Vec<&str> = Vec::with_capacity(9);
        if l.index.is_none() {
            // Before the singing starts: the pause, then what comes.
            texts.push("");
        }
        let last = (current + 6).min(l.lines.len());
        texts.extend(
            l.lines[first.min(last)..last]
                .iter()
                .map(|x| x.text.as_str()),
        );
        let at = current - first;
        let layouts: Vec<IDWriteTextLayout> = texts
            .iter()
            .filter_map(|t| {
                let t = if t.trim().is_empty() {
                    layout::LYRICS_PAUSE
                } else {
                    t
                };
                unsafe {
                    self.text
                        .dw
                        .CreateTextLayout(&utf16(t), &self.text.lines, col.width, 400.0)
                        .ok()
                }
            })
            .collect();
        if layouts.len() != texts.len() {
            return;
        }
        let heights: Vec<f32> = layouts
            .iter()
            .map(|t| unsafe {
                let mut m = DWRITE_TEXT_METRICS::default();
                let _ = t.GetMetrics(&mut m);
                m.height
            })
            .collect();
        let tops = layout::lyrics_tops(&heights, at, l.scroll);
        // While it scrolls, the focus passes from the line before to this.
        let focus = at as f32 - l.scroll;
        unsafe {
            self.dc
                .PushAxisAlignedClip(&rf(col), D2D1_ANTIALIAS_MODE_ALIASED);
            for (i, t) in layouts.iter().enumerate() {
                let a = layout::lyrics_alpha(i as f32 - focus)
                    * layout::lyrics_edge(tops[i], heights[i])
                    * alpha;
                if a <= 0.01 {
                    continue;
                }
                self.ink.SetColor(&white(a));
                self.dc.DrawTextLayout(
                    v(Point {
                        x: col.x,
                        y: col.y + tops[i],
                    }),
                    t,
                    &self.ink,
                    D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT,
                );
            }
            self.dc.PopAxisAlignedClip();
        }
    }

    fn draw_peek(&mut self, s: &Scene, dx: f32, alpha: f32) {
        if let Some(n) = s.notice {
            let icon = shift(layout::PEEK_ART, dx);
            self.tinted(n.icon, &self.text.icon, icon, n.tint, alpha);
            let spans = [
                (n.title.to_string(), false),
                (format!("  {}", n.detail), true),
            ];
            let text = if n.level.is_some() {
                glance::LEVEL_TEXT
            } else {
                layout::PEEK_TEXT
            };
            self.song(&spans, &self.text.peek, shift(text, dx), alpha);
            if let Some(level) = n.level {
                let bar = shift(glance::LEVEL_BAR, dx);
                self.rounded(bar, bar.height / 2.0, 0.2 * alpha);
                let done = Rect {
                    width: bar.width * level.clamp(0.0, 1.0),
                    ..bar
                };
                if done.width > 0.0 {
                    self.tinted_rounded(done, bar.height / 2.0, n.tint, 0.95 * alpha);
                }
            }
            return;
        }
        let Some(t) = s.track else { return };
        self.draw_art(t, shift(layout::PEEK_ART, dx), 7.0, alpha, s.scale);
        let spans = match s.lyrics {
            Some(l) if s.glance => vec![(l.current().to_string(), false)],
            _ => s.song.spans(&t.title, &t.artist),
        };
        self.song(&spans, &self.text.peek, shift(layout::PEEK_TEXT, dx), alpha);
        self.bars(t, shift(layout::PEEK_BARS, dx), alpha);
    }

    fn draw_closed(&mut self, s: &Scene, dx: f32, alpha: f32) {
        let Some(t) = s.track else {
            // Nothing playing: the focus timer, while a session is on.
            if let Some(time) = s.closed_timer {
                let h = layout::CLOSED_TEXT.y * 2.0 + layout::CLOSED_TEXT.height;
                let icon = Rect {
                    x: dx + 18.0,
                    y: 0.0,
                    width: 20.0,
                    height: h,
                };
                self.tinted("\u{E916}", &self.text.tab, icon, s.focus.tint(), alpha);
                let text = Rect {
                    x: dx + 44.0,
                    y: 0.0,
                    width: CLOSED_W - 62.0,
                    height: h,
                };
                self.text(time, &self.text.closed, text, 0.95 * alpha);
            }
            return;
        };
        self.draw_art(t, shift(layout::SMALL_ART, dx), 5.0, alpha, s.scale);
        let spans = s.song.spans(&t.title, &t.artist);
        let text = shift(layout::CLOSED_TEXT, dx);
        self.song(&spans, &self.text.closed, text, alpha);
        self.bars(t, shift(layout::BARS, dx), alpha);
    }

    /// The bars are composition visuals (see `bars.rs`): here we only note
    /// where they go, the most visible view winning.
    fn bars(&mut self, t: &Track, bars: Rect, alpha: f32) {
        // Where the view's entrance puts them; their size stays, so their
        // surface is never painted again mid-motion.
        let (cx, k, drop) = self.view;
        let bars = Rect {
            x: cx + (bars.x + bars.width / 2.0 - cx) * k - bars.width / 2.0,
            y: (bars.y + bars.height / 2.0) * k - drop - bars.height / 2.0,
            ..bars
        };
        if self.bars_at.is_none_or(|(_, a, _)| alpha > a) {
            self.bars_at = Some((bars, alpha, t.playing));
        }
    }

    /// Places the next view for its entrance at visibility `a`, scaled
    /// about (`cx`, 0) of the content; gives its opacity.
    fn enter(&mut self, base: &Matrix3x2, cx: f32, a: f32) -> f32 {
        let (opacity, k, drop) = layout::entrance(a);
        self.view = (cx, k, drop);
        unsafe {
            self.dc
                .SetTransform(&(affine(k, cx * (1.0 - k), -drop) * *base));
        }
        opacity
    }

    /// The cover's color rising from the bottom of the body (px, from the
    /// body's top-left).
    fn glow(&self, geo: &ID2D1PathGeometry, height: f32, accent: [f32; 3]) -> Result<()> {
        let [r, g, b] = accent;
        let at = |position, a| D2D1_GRADIENT_STOP {
            position,
            color: D2D1_COLOR_F { r, g, b, a },
        };
        unsafe {
            let stops = ID2D1RenderTarget::CreateGradientStopCollection(
                &self.dc,
                &[at(0.0, 0.0), at(1.0, 0.32)],
                D2D1_GAMMA_2_2,
                D2D1_EXTEND_MODE_CLAMP,
            )?;
            let brush = self.dc.CreateLinearGradientBrush(
                &D2D1_LINEAR_GRADIENT_BRUSH_PROPERTIES {
                    startPoint: Vector2 {
                        X: 0.0,
                        Y: height * 0.35,
                    },
                    endPoint: Vector2 { X: 0.0, Y: height },
                },
                None,
                &stops,
            )?;
            self.dc.FillGeometry(geo, &brush, None::<&ID2D1Brush>);
        }
        Ok(())
    }

    /// Redraws the whole window: the island's outline at its body, its
    /// content on top, on a transparent background.
    pub fn draw(&mut self, s: &Scene) -> Result<()> {
        let geo = self.geometry(s.outline)?;
        let surface = self.surface(s.width.max(1), s.height.max(1))?;
        let side = (layout::ART.width * s.scale).ceil();
        self.accent = s
            .track
            .and_then(|t| t.art.as_ref())
            .and_then(|b| self.art(b, side))
            .map(|a| a.accent);
        let (sw, sh) = (s.width.max(1) as i32, s.height.max(1) as i32);
        let now = RECT {
            left: (s.body.x.floor() as i32 - 1).clamp(0, sw - 1),
            top: (s.body.y.floor() as i32 - 1).clamp(0, sh - 1),
            right: ((s.body.x + s.body.width).ceil() as i32 + 1).clamp(1, sw),
            bottom: ((s.body.y + s.body.height).ceil() as i32 + 1).clamp(1, sh),
        };
        let dirty = match self.drawn {
            Some(d) => RECT {
                left: d.left.min(now.left),
                top: d.top.min(now.top),
                right: d.right.max(now.right),
                bottom: d.bottom.max(now.bottom),
            },
            None => RECT {
                left: 0,
                top: 0,
                right: sw,
                bottom: sh,
            },
        };
        unsafe {
            let mut offset = POINT::default();
            let dxgi: IDXGISurface = surface.BeginDraw(Some(&dirty), &mut offset)?;
            let drawn = (|| {
                let props = D2D1_BITMAP_PROPERTIES1 {
                    pixelFormat: D2D1_PIXEL_FORMAT {
                        format: DXGI_FORMAT_B8G8R8A8_UNORM,
                        alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                    },
                    dpiX: 96.0,
                    dpiY: 96.0,
                    bitmapOptions: D2D1_BITMAP_OPTIONS_TARGET | D2D1_BITMAP_OPTIONS_CANNOT_DRAW,
                    ..Default::default()
                };
                let bitmap = self.dc.CreateBitmapFromDxgiSurface(&dxgi, Some(&props))?;
                self.dc.SetTarget(&bitmap);
                self.dc.BeginDraw();
                // The update area can live inside a bigger atlas: its
                // top-left is at `offset`. From here on, window px.
                let ox = (offset.x - dirty.left) as f32;
                let oy = (offset.y - dirty.top) as f32;
                self.dc.SetTransform(&Matrix3x2::translation(ox, oy));
                self.dc.PushAxisAlignedClip(
                    &D2D_RECT_F {
                        left: dirty.left as f32,
                        top: dirty.top as f32,
                        right: dirty.right as f32,
                        bottom: dirty.bottom as f32,
                    },
                    D2D1_ANTIALIAS_MODE_ALIASED,
                );
                self.dc.Clear(Some(&D2D1_COLOR_F::default()));
                let (bx, by) = (ox + s.body.x, oy + s.body.y);
                self.dc.SetTransform(&Matrix3x2::translation(bx, by));
                self.dc.FillGeometry(&geo, &self.fill, None::<&ID2D1Brush>);
                if let Some(accent) = self.accent {
                    self.glow(&geo, s.body.height, accent)?;
                }

                // The content, in DIPs, clipped to the body.
                let base = affine(s.scale, bx + s.inset, by);
                self.dc.SetTransform(&base);
                self.dc.PushAxisAlignedClip(
                    &D2D_RECT_F {
                        left: 0.0,
                        top: 0.0,
                        right: s.size.0,
                        bottom: s.size.1,
                    },
                    D2D1_ANTIALIAS_MODE_ALIASED,
                );
                let [closed, peek, open] = s.alpha;
                let cx = s.size.0 / 2.0;
                if closed > 0.0 {
                    let w = if s.track.is_some() {
                        layout::CLOSED_MEDIA_WIDTH
                    } else {
                        CLOSED_W
                    };
                    let a = self.enter(&base, cx, closed);
                    self.draw_closed(s, (s.size.0 - w) / 2.0, a);
                }
                if peek > 0.0 {
                    let a = self.enter(&base, cx, peek);
                    self.draw_peek(s, (s.size.0 - layout::PEEK.0) / 2.0, a);
                }
                if open > 0.0 {
                    let a = self.enter(&base, cx, open);
                    let w = s.open.width;
                    self.draw_open(s, (s.size.0 - w) / 2.0, a);
                }
                self.view = (0.0, 1.0, 0.0);
                self.dc.PopAxisAlignedClip();

                self.dc.PopAxisAlignedClip();
                let ended = self.dc.EndDraw(None, None);
                self.dc.SetTarget(None::<&ID2D1Image>);
                ended
            })();
            surface.EndDraw()?;
            drawn?;
            self.drawn = Some(now);
            match self.bars_at.take() {
                Some((r, alpha, playing)) => {
                    let k = s.scale;
                    let px = Rect {
                        x: s.body.x + s.inset + r.x * k,
                        y: s.body.y + r.y * k,
                        width: r.width * k,
                        height: r.height * k,
                    };
                    let color = self.accent.unwrap_or([1.0; 3]);
                    self.bars
                        .update(&self.dcomp, &self.dc, px, k, color, alpha, playing)?;
                }
                None => self.bars.hide()?,
            }
            self.dcomp.Commit()
        }
    }

    pub fn menu_target(&self, hwnd: HWND) -> Result<MenuTarget> {
        unsafe {
            let target = self.dcomp.CreateTargetForHwnd(hwnd, true)?;
            let visual = self.dcomp.CreateVisual()?;
            target.SetRoot(&visual)?;
            Ok(MenuTarget {
                _target: target,
                visual,
                surface: None,
            })
        }
    }

    /// The whole menu, every time: it is small and drawn only on a change.
    pub fn draw_menu(&mut self, t: &mut MenuTarget, m: &MenuView) -> Result<()> {
        let (w, h) = (m.width.max(1), m.height.max(1));
        let surface = match &t.surface {
            Some((s, sw, sh)) if (*sw, *sh) == (w, h) => s.clone(),
            _ => unsafe {
                let s = self.dcomp.CreateSurface(
                    w,
                    h,
                    DXGI_FORMAT_B8G8R8A8_UNORM,
                    DXGI_ALPHA_MODE_PREMULTIPLIED,
                )?;
                t.visual.SetContent(&s)?;
                t.surface = Some((s.clone(), w, h));
                s
            },
        };
        unsafe {
            let mut offset = POINT::default();
            let dxgi: IDXGISurface = surface.BeginDraw(None, &mut offset)?;
            let drawn = (|| {
                let props = D2D1_BITMAP_PROPERTIES1 {
                    pixelFormat: D2D1_PIXEL_FORMAT {
                        format: DXGI_FORMAT_B8G8R8A8_UNORM,
                        alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                    },
                    dpiX: 96.0,
                    dpiY: 96.0,
                    bitmapOptions: D2D1_BITMAP_OPTIONS_TARGET | D2D1_BITMAP_OPTIONS_CANNOT_DRAW,
                    ..Default::default()
                };
                let bitmap = self.dc.CreateBitmapFromDxgiSurface(&dxgi, Some(&props))?;
                self.dc.SetTarget(&bitmap);
                self.dc.BeginDraw();
                let (ox, oy) = (offset.x as f32, offset.y as f32);
                self.dc.SetTransform(&Matrix3x2::translation(ox, oy));
                self.dc.PushAxisAlignedClip(
                    &D2D_RECT_F {
                        left: 0.0,
                        top: 0.0,
                        right: w as f32,
                        bottom: h as f32,
                    },
                    D2D1_ANTIALIAS_MODE_ALIASED,
                );
                self.dc.Clear(Some(&D2D1_COLOR_F::default()));
                // From here on, DIPs.
                let k = m.scale;
                self.dc.SetTransform(&affine(k, ox, oy));
                self.menu_body(m, w as f32 / k, h as f32 / k);
                self.dc.PopAxisAlignedClip();
                let ended = self.dc.EndDraw(None, None);
                self.dc.SetTarget(None::<&ID2D1Image>);
                ended
            })();
            surface.EndDraw()?;
            drawn?;
            self.dcomp.Commit()
        }
    }

    fn menu_body(&self, m: &MenuView, w: f32, h: f32) {
        let panel = D2D1_ROUNDED_RECT {
            rect: D2D_RECT_F {
                left: 0.5,
                top: 0.5,
                right: w - 0.5,
                bottom: h - 0.5,
            },
            radiusX: menu::RADIUS,
            radiusY: menu::RADIUS,
        };
        unsafe {
            // A dark grey, softer than the island's black, like the
            // system's own flyouts; a light rim keeps its edge.
            self.ink.SetColor(&MENU_PANEL);
            self.dc.FillRoundedRectangle(&panel, &self.ink);
            self.ink.SetColor(&white(0.16));
            self.dc
                .DrawRoundedRectangle(&panel, &self.ink, 1.0 / m.scale, None);
        }
        let head = Rect {
            x: menu::TEXT_X,
            y: menu::PAD,
            width: w - 2.0 * menu::TEXT_X,
            height: menu::HEADER,
        };
        self.text(m.title, &self.text.menu_head, head, 0.95);
        self.text(m.version, &self.text.time_end, head, 0.4);
        let kinds: Vec<menu::Kind> = m.rows.iter().map(|r| r.kind).collect();
        for (i, (row, r)) in m.rows.iter().zip(menu::rows(&kinds)).enumerate() {
            if row.kind == menu::Kind::Separator {
                let line = Rect {
                    x: menu::TEXT_X - 2.0,
                    y: r.y + r.height / 2.0,
                    width: w - 2.0 * (menu::TEXT_X - 2.0),
                    height: 1.0 / m.scale,
                };
                self.rounded(line, 0.0, 0.1);
                continue;
            }
            let a = if row.enabled { 1.0 } else { 0.35 };
            if m.hover == Some(i) && row.enabled {
                self.rounded(r, menu::ROW_RADIUS, 0.09);
            }
            let switch = (row.kind == menu::Kind::Toggle).then(|| menu::switch(r));
            let label = Rect {
                x: menu::TEXT_X,
                y: r.y,
                width: switch.map_or(r.x + r.width, |s| s.x - 8.0) - menu::TEXT_X,
                height: r.height,
            };
            self.text(row.label, &self.text.menu, label, 0.92 * a);
            if let Some(s) = switch {
                let radius = s.height / 2.0;
                let knob = menu::knob(s, if row.on { 1.0 } else { 0.0 });
                if row.on {
                    // On: a white track, the knob in the panel's grey.
                    self.rounded(s, radius, 0.92 * a);
                    unsafe {
                        self.ink.SetColor(&MENU_PANEL);
                        self.dc.FillEllipse(&ellipse(knob), &self.ink);
                    }
                } else {
                    self.rounded(s, radius, 0.16 * a);
                    unsafe {
                        self.ink.SetColor(&white(0.85 * a));
                        self.dc.FillEllipse(&ellipse(knob), &self.ink);
                    }
                }
            }
        }
    }
}

fn ellipse(r: Rect) -> D2D1_ELLIPSE {
    D2D1_ELLIPSE {
        point: v(Point {
            x: r.x + r.width / 2.0,
            y: r.y + r.height / 2.0,
        }),
        radiusX: r.width / 2.0,
        radiusY: r.height / 2.0,
    }
}
