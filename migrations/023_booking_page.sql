-- Link each booking unit to the CMS page that presents it (lot 2.1).
-- page_path is the doxyde page path (e.g. /stars) so stay-search results can
-- deep-link to the rich apartment page, and that page can quote/book its own
-- unit directly. Single ALTER statement (the migration runner tolerates the
-- duplicate-column error if it has already been applied).
ALTER TABLE booking_listing ADD COLUMN page_path TEXT
