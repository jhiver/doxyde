// Master catalog of UI labels (layer 1 of i18n).
//
// Each entry is (dotted key, canonical English source). The English source is
// authoritative; other languages are machine-translated on demand and cached in
// the per-site `ui_label_cache` table (see `ui_labels`). Keep keys stable —
// they are referenced from templates as `labels["key"]`.

/// (key, English source) pairs. Start minimal; grow as templates need labels.
pub const UI_LABELS: &[(&str, &str)] = &[
    // Navigation / chrome
    ("nav.home", "Home"),
    ("lang.switch", "Language"),
    // Booking widget (hero_booking + apartment templates)
    ("booking.search", "Search"),
    ("booking.checkin", "Check-in"),
    ("booking.checkout", "Check-out"),
    ("booking.guests", "Guests"),
    ("booking.adults", "Adults"),
    ("booking.children", "Children"),
    ("booking.book", "Book"),
    ("booking.check_availability", "Check availability"),
    ("booking.select_date", "Select date"),
    ("booking.add_date", "Add date"),
    // Guest-count option: singular noun ("1 Guest"); plural reuses booking.guests.
    ("booking.guest_one", "Guest"),
    ("booking.show_all_photos", "Show all photos"),
];

/// Canonical English source for a key, if known.
pub fn source(key: &str) -> Option<&'static str> {
    UI_LABELS.iter().find(|(k, _)| *k == key).map(|(_, v)| *v)
}
