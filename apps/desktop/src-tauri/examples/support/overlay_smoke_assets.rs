//! A dedicated local page: no production React code or recording initialization.
use std::borrow::Cow;
use tauri::utils::assets::{AssetKey, AssetsIter, CspHash};

pub struct SmokeAssets;
impl tauri::Assets<tauri::Wry> for SmokeAssets {
    fn get(&self, key: &AssetKey) -> Option<Cow<'_, [u8]>> {
        match key.as_ref() {
            "index.html" => Some(Cow::Borrowed(b"<!doctype html><html><head><meta charset='utf-8'></head><body></body></html>")),
            _ => None,
        }
    }
    fn iter(&self) -> Box<AssetsIter<'_>> { Box::new(std::iter::empty()) }
    fn csp_hashes(&self, _: &AssetKey) -> Box<dyn Iterator<Item = CspHash<'_>> + '_> {
        Box::new(std::iter::empty())
    }
}
