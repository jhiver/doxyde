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
    // Search results + booking flow (lot 2)
    ("booking.results_title", "Available stays"),
    ("booking.no_results", "No availability for these dates. Try different dates."),
    ("booking.sister_house", "Also available at our sister house"),
    ("booking.multi_stay_note", "Combination of stays — each leg is booked separately."),
    ("booking.estimated", "estimated"),
    ("booking.nights", "nights"),
    ("booking.from_label", "From"),
    ("booking.to_label", "To"),
    ("booking.book_now", "Book now"),
    ("booking.your_details", "Your details"),
    ("booking.first_name", "First name"),
    ("booking.last_name", "Last name"),
    ("booking.email", "Email"),
    ("booking.phone", "Phone"),
    ("booking.special_requests", "Special requests"),
    ("booking.confirm_booking", "Confirm booking"),
    ("booking.total_price", "Total price"),
    ("booking.unavailable", "These dates are no longer available."),
    ("booking.search_again", "Search again"),
    ("booking.invalid_dates", "Please pick a check-out date after your check-in date."),
    ("booking.no_card_required", "No credit card required"),
    ("booking.pay_later", "Reserve now — no card needed. We'll email you payment instructions after you book."),
    ("booking.not_configured", "Booking is not configured for this site yet."),
    (
        "booking.service_error",
        "The booking service is temporarily unavailable. Please try again shortly.",
    ),
    ("booking.confirmed_title", "Booking confirmed"),
    (
        "booking.confirmed_intro",
        "Thank you — your reservation is confirmed. You will receive an email with payment instructions shortly.",
    ),
    ("booking.confirmation_code", "Confirmation code"),
    (
        "booking.booking_error",
        "We could not complete your booking. Please try again or contact us.",
    ),
];

/// Canonical English source for a key, if known.
pub fn source(key: &str) -> Option<&'static str> {
    UI_LABELS.iter().find(|(k, _)| *k == key).map(|(_, v)| *v)
}
