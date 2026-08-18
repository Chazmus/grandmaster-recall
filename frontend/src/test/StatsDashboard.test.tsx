import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { StatsDashboard } from '../components/StatsDashboard';
import { StatsSummary } from '../types';

const mockStats: StatsSummary = {
  total_puzzles: 45,
  due_today: 7,
  mastered_puzzles: 12,
  total_reviews: 60,
  retention_rate: 91.5,
  blunders_count: 18,
  mistakes_count: 15,
  inaccuracies_count: 12,
  tactical_tag_breakdown: [
    { tag: 'Pin', count: 14, success_rate: 85.7 },
    { tag: 'Fork', count: 11, success_rate: 72.7 },
  ],
  top_blundered_openings: [
    { opening_name: "King's Indian Defense", blunder_count: 8 },
    { opening_name: 'Caro-Kann Defense', blunder_count: 5 },
  ],
};

describe('StatsDashboard Component', () => {
  it('renders analytics metrics and tag breakdown correctly', () => {
    render(<StatsDashboard stats={mockStats} username="cbailey" />);

    expect(screen.getByText(/Tactical Analytics & Progress for/i)).toBeInTheDocument();
    expect(screen.getByText('91.5%')).toBeInTheDocument();
    expect(screen.getByText('12 Mastered')).toBeInTheDocument();
    expect(screen.getByText('Pin')).toBeInTheDocument();
    expect(screen.getByText("King's Indian Defense")).toBeInTheDocument();
  });
});
