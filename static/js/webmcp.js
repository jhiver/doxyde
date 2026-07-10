(() => {
  const ctx = document.modelContext ?? navigator.modelContext;
  if (!ctx?.registerTool) return;

  const isValidDate = (str) => {
    if (typeof str !== 'string') return false;
    return /^\d{4}-\d{2}-\d{2}$/.test(str);
  };

  const safeFetchJson = async (url, options = {}) => {
    try {
      const response = await fetch(url, options);
      if (!response.ok) {
        return JSON.stringify({ status: "error", code: "http_" + response.status });
      }
      return await response.text();
    } catch (e) {
      return JSON.stringify({ status: "error", code: "network" });
    }
  };

  ctx.registerTool({
    name: "search_stays",
    description: "Search for available stays. Results include details such as total price, capacity, and page_url (which maps to a rich details page). After finding a suitable listing, call get_stay_quote to retrieve a precise quote, and finally book_stay to perform the reservation.",
    inputSchema: {
      type: "object",
      properties: {
        checkin: { type: "string", description: "The check-in date in YYYY-MM-DD format" },
        checkout: { type: "string", description: "The check-out date in YYYY-MM-DD format" },
        adults: { type: "integer", default: 2, description: "Number of adults" },
        children: { type: "integer", default: 0, description: "Number of children" }
      },
      required: ["checkin", "checkout"]
    },
    annotations: { readOnlyHint: true },
    execute: async (input) => {
      if (!isValidDate(input.checkin) || !isValidDate(input.checkout)) {
        return JSON.stringify({ status: "error", code: "invalid_input" });
      }
      const url = new URL("/.stay", window.location.origin);
      url.searchParams.append("from", input.checkin);
      url.searchParams.append("to", input.checkout);
      url.searchParams.append("adults", input.adults !== undefined ? input.adults : 2);
      url.searchParams.append("kids", input.children !== undefined ? input.children : 0);
      url.searchParams.append("format", "json");
      url.searchParams.append("utm_source", "webmcp");
      return safeFetchJson(url, { headers: { "Accept": "application/json" } });
    }
  });

  ctx.registerTool({
    name: "get_stay_quote",
    description: "Retrieve a detailed quote and get the calendar of blocked dates for a specific listing and date range. Must be called before creating a reservation.",
    inputSchema: {
      type: "object",
      properties: {
        listing_id: { type: "integer", description: "The ID of the listing, retrieved from legs[].listing_id in the search results" },
        checkin: { type: "string", description: "The check-in date in YYYY-MM-DD format" },
        checkout: { type: "string", description: "The check-out date in YYYY-MM-DD format" },
        adults: { type: "integer", default: 2, description: "Number of adults" },
        children: { type: "integer", default: 0, description: "Number of children" }
      },
      required: ["listing_id", "checkin", "checkout"]
    },
    annotations: { readOnlyHint: true },
    execute: async (input) => {
      if (!isValidDate(input.checkin) || !isValidDate(input.checkout)) {
        return JSON.stringify({ status: "error", code: "invalid_input" });
      }
      const url = new URL("/.book", window.location.origin);
      url.searchParams.append("listing", input.listing_id);
      url.searchParams.append("from", input.checkin);
      url.searchParams.append("to", input.checkout);
      url.searchParams.append("adults", input.adults !== undefined ? input.adults : 2);
      url.searchParams.append("kids", input.children !== undefined ? input.children : 0);
      url.searchParams.append("format", "json");
      url.searchParams.append("utm_source", "webmcp");
      return safeFetchJson(url, { headers: { "Accept": "application/json" } });
    }
  });

  ctx.registerTool({
    name: "book_stay",
    description: "Creates a REAL confirmed reservation that blocks the calendar. Only call after the user has explicitly confirmed listing, dates and guest details. No payment is taken at booking time; payment instructions are emailed.",
    inputSchema: {
      type: "object",
      properties: {
        listing_id: { type: "integer", description: "The ID of the listing" },
        checkin: { type: "string", description: "The check-in date in YYYY-MM-DD format" },
        checkout: { type: "string", description: "The check-out date in YYYY-MM-DD format" },
        first_name: { type: "string", description: "First name of the primary guest" },
        last_name: { type: "string", description: "Last name of the primary guest" },
        email: { type: "string", description: "Email address of the primary guest" },
        adults: { type: "integer", default: 2, description: "Number of adults" },
        children: { type: "integer", default: 0, description: "Number of children" },
        phone: { type: "string", description: "Optional phone number" },
        note: { type: "string", description: "Optional message/note for the host" }
      },
      required: ["listing_id", "checkin", "checkout", "first_name", "last_name", "email"]
    },
    annotations: { readOnlyHint: false },
    execute: async (input) => {
      if (!isValidDate(input.checkin) || !isValidDate(input.checkout)) {
        return JSON.stringify({ status: "error", code: "invalid_input" });
      }
      const url = new URL("/.book", window.location.origin);
      const formData = new URLSearchParams();
      formData.append("listing_id", input.listing_id);
      formData.append("from", input.checkin);
      formData.append("to", input.checkout);
      formData.append("first_name", input.first_name);
      formData.append("last_name", input.last_name);
      formData.append("email", input.email);
      formData.append("adults", input.adults !== undefined ? input.adults : 2);
      formData.append("children", input.children !== undefined ? input.children : 0);
      if (input.phone !== undefined) formData.append("phone", input.phone);
      if (input.note !== undefined) formData.append("note", input.note);
      formData.append("format", "json");
      formData.append("utm_source", "webmcp");

      return safeFetchJson(url, {
        method: "POST",
        headers: {
          "Content-Type": "application/x-www-form-urlencoded",
          "Accept": "application/json"
        },
        body: formData.toString()
      });
    }
  });
})();
