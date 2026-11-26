//! Native-resolution page tiles drawn over the page view's lightweight previews.

use std::collections::HashSet;

use craft::pages::ui::{PageBody, PlacedPage};
use craft::ui::primitives::textures::{
    Priority, Revision, TextureBudget, TextureStatistics, TextureStoreOptions, ThumbRender, ThumbRequest, ThumbSource, ThumbnailOptions, Thumbnails,
};
use egui::emath::GuiRounding;
use egui::{Color32, Rect, Ui, pos2};

use super::source::WordPages;

const TILE_PIXELS: u32 = 512;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct Tile {
    page: usize,
    size: [u32; 2],
    x: u32,
    y: u32,
}

impl ThumbSource<Tile> for WordPages {
    fn revision(&self, tile: &Tile, _: &ThumbRequest) -> Revision {
        self.revision_for(tile.page)
    }

    fn render(&self, tile: &Tile, request: &ThumbRequest) -> ThumbRender {
        self.render_picture(tile.page, request.side, request.region)
    }
}

pub(super) fn show(ui: &Ui, body: &PageBody<'_>, source: &WordPages) -> TextureStatistics {
    if body.pass.is_none() {
        return TextureStatistics::default();
    }
    let ctx = ui.ctx();
    let ppp = ctx.pixels_per_point();
    // Retention follows the viewport. Leave room in the store for the visible tiles on large displays.
    let store = TextureStoreOptions::pages().budget(TextureBudget::new(128 * 1024 * 1024, 256));
    let lanes = usize::from(!cfg!(target_arch = "wasm32"));
    let options = ThumbnailOptions::new().store(store).max_side(TILE_PIXELS).lanes(lanes);
    let tiles = Thumbnails::in_context(ctx, ui.scope_id().with("page-tiles"), options);
    let clip = body.view.rect.intersect(ui.clip_rect());
    let mut visible = HashSet::new();
    for page in body.pages {
        paint_page(ui, source, &tiles, page, clip, ppp, &mut visible);
    }
    tiles.retain(ctx, |tile| visible.contains(tile));
    tiles.cache().statistics(ctx)
}

fn paint_page(ui: &Ui, source: &WordPages, tiles: &Thumbnails<Tile>, page: &PlacedPage, clip: Rect, ppp: f32, visible: &mut HashSet<Tile>) {
    let rect = Rect::from_min_size(page.rect.min.round_to_pixels(ppp), page.rect.size().round_to_pixels(ppp));
    let band = Rect::from_x_y_ranges(rect.x_range(), (page.rect.top() + page.band.0)..=(page.rect.top() + page.band.1));
    let clip = clip.intersect(band);
    if !clip.is_positive() || !rect.is_positive() {
        return;
    }
    let size = [(rect.width() * ppp).round() as u32, (rect.height() * ppp).round() as u32];
    if size.contains(&0) || size[0].max(size[1]) <= TILE_PIXELS {
        return;
    }
    let tile_range = |from: f32, to: f32, start: f32, size: u32| {
        let first = ((from - start) * ppp).floor() as u32;
        let end = ((to - start) * ppp).ceil() as u32;
        first / TILE_PIXELS..end.min(size).div_ceil(TILE_PIXELS)
    };
    let columns = tile_range(clip.left(), clip.right(), rect.left(), size[0]);
    let rows = tile_range(clip.top(), clip.bottom(), rect.top(), size[1]);
    let painter = ui.painter().with_clip_rect(clip);
    for y in rows {
        for x in columns.clone() {
            let tile = Tile { page: page.page, size, x, y };
            visible.insert(tile);
            let left = x * TILE_PIXELS;
            let top = y * TILE_PIXELS;
            let right = left.saturating_add(TILE_PIXELS).min(size[0]);
            let bottom = top.saturating_add(TILE_PIXELS).min(size[1]);
            let fraction = |x, y| pos2(x as f32 / size[0] as f32, y as f32 / size[1] as f32);
            let on_screen = |x, y| rect.min + egui::vec2(x as f32 / ppp, y as f32 / ppp);
            let region = Rect::from_min_max(fraction(left, top), fraction(right, bottom));
            let side = (right - left).max(bottom - top);
            let request = ThumbRequest::for_cell(side as f32 / ppp, ppp).region(region);
            if let Some(texture) = tiles.state(ui.ctx(), source, &tile, &request, Priority::VISIBLE).shown() {
                let target = Rect::from_min_max(on_screen(left, top), on_screen(right, bottom));
                painter.image(texture.id, target, texture.uv(), Color32::WHITE);
            }
        }
    }
}
