import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('errors');

// Test configuration
export const options = {
  stages: [
    { duration: '30s', target: 10 },   // Ramp up to 10 users
    { duration: '1m', target: 50 },    // Ramp up to 50 users
    { duration: '2m', target: 100 },   // Ramp up to 100 users
    { duration: '2m', target: 100 },   // Stay at 100 users
    { duration: '1m', target: 0 },     // Ramp down to 0 users
  ],
  thresholds: {
    http_req_duration: ['p(95)<500'],  // 95% of requests must complete below 500ms
    errors: ['rate<0.05'],             // Error rate must be below 5%
  },
};

const BASE_URL = 'http://localhost:3000';

export default function () {
  // Test homepage
  let res = http.get(`${BASE_URL}/`);
  check(res, {
    'homepage status is 200': (r) => r.status === 200,
    'homepage loads quickly': (r) => r.timings.duration < 200,
  });
  errorRate.add(res.status !== 200);

  sleep(1);

  // Test blog listing
  res = http.get(`${BASE_URL}/blog`);
  check(res, {
    'blog status is 200': (r) => r.status === 200,
    'blog loads quickly': (r) => r.timings.duration < 300,
  });
  errorRate.add(res.status !== 200);

  sleep(1);

  // Test documentation page
  res = http.get(`${BASE_URL}/documentation`);
  check(res, {
    'docs status is 200': (r) => r.status === 200,
    'docs loads quickly': (r) => r.timings.duration < 300,
  });
  errorRate.add(res.status !== 200);

  sleep(2);
}