import { render, screen } from '@testing-library/react';
import { describe, expect, test } from 'vitest';

import ConverterForm from './ConverterForm';

// Smoke test only — assert the form renders with its essential controls.
// The real behavior (POST -> /api/v2/convert, result rendering, copy button)
// is verified by manual browser smoke against the local backend; mocking
// fetch + assertion-heavy tests would pin to current copy and break on UI
// tweaks without catching anything the browser smoke wouldn't.
describe('ConverterForm', () => {
  test('renders the URL input and submit button', () => {
    render(<ConverterForm />);

    const input = screen.getByPlaceholderText(/google\.com\/amp/i);
    expect(input.tagName).toBe('INPUT');
    expect(input.getAttribute('type')).toBe('url');

    const submit = screen.getByRole('button', { name: /submit url/i });
    expect(submit).not.toBeNull();
  });
});
