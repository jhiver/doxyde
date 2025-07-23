import http from 'k6/http';
import { check } from 'k6';
import { Rate } from 'k6/metrics';

const errorRate = new Rate('mcp_errors');

export const options = {
  scenarios: {
    mcp_constant_load: {
      executor: 'constant-arrival-rate',
      rate: 100,           // 100 requests per second
      timeUnit: '1s',
      duration: '2m',
      preAllocatedVUs: 50,
      maxVUs: 100,
    },
  },
  thresholds: {
    http_req_duration: ['p(95)<1000'], // 95% of MCP requests under 1s
    mcp_errors: ['rate<0.01'],         // Less than 1% error rate
  },
};

const BASE_URL = 'http://localhost:3000';
const MCP_TOKEN = 'your-mcp-token-here'; // Set this to a valid token

// MCP request templates
const mcp_requests = [
  // List pages
  {
    jsonrpc: '2.0',
    method: 'mcp__doxyde__list_pages',
    params: {},
    id: 1,
  },
  // Get page
  {
    jsonrpc: '2.0',
    method: 'mcp__doxyde__get_page',
    params: { page_id: 1 },
    id: 2,
  },
  // Search pages
  {
    jsonrpc: '2.0',
    method: 'mcp__doxyde__search_pages',
    params: { query: 'documentation' },
    id: 3,
  },
];

export default function () {
  // Randomly select an MCP request
  const request = mcp_requests[Math.floor(Math.random() * mcp_requests.length)];
  
  const params = {
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${MCP_TOKEN}`,
    },
  };

  const res = http.post(
    `${BASE_URL}/.mcp/v1/jsonrpc`,
    JSON.stringify(request),
    params
  );

  const success = check(res, {
    'MCP status is 200': (r) => r.status === 200,
    'MCP response is valid JSON': (r) => {
      try {
        const json = JSON.parse(r.body);
        return json.jsonrpc === '2.0';
      } catch {
        return false;
      }
    },
    'MCP no error': (r) => {
      try {
        const json = JSON.parse(r.body);
        return !json.error;
      } catch {
        return false;
      }
    },
    'MCP response time acceptable': (r) => r.timings.duration < 500,
  });

  errorRate.add(!success);
}