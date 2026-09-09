#[derive(Default)]
pub(crate) struct ShortcutEdge(std::sync::atomic::AtomicBool);

impl ShortcutEdge {
    pub(crate) fn accept(&self, pressed: bool) -> bool {
        let was_pressed = self.0.swap(pressed, std::sync::atomic::Ordering::SeqCst);
        pressed && !was_pressed
    }
}

#[cfg(test)]
mod tests {
    use super::ShortcutEdge;
    #[test]
    fn one_toggle_per_press_even_with_repeats() {
        let edge = ShortcutEdge::default();
        assert!(!edge.accept(false));
        assert!(edge.accept(true));
        for _ in 0..20 {
            assert!(!edge.accept(true));
        }
        assert!(!edge.accept(false));
        assert!(edge.accept(true));
        assert!(!edge.accept(false));
    }
}
