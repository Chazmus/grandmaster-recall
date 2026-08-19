import { describe, it, expect } from 'vitest';
import { render } from '@testing-library/react';
import { Chessboard } from '../components/Chessboard';

describe('Chessboard Component', () => {
  it('renders correctly with white orientation', () => {
    const { container } = render(
      <Chessboard
        fen="rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
        orientation="white"
      />
    );

    const wrap = container.querySelector('.cg-wrap');
    expect(wrap).toHaveClass('orientation-white');

    const ranksCoords = container.querySelector('coords.ranks');
    expect(ranksCoords).toBeInTheDocument();
    expect(ranksCoords).not.toHaveClass('black');

    const filesCoords = container.querySelector('coords.files');
    expect(filesCoords).toBeInTheDocument();
    expect(filesCoords).not.toHaveClass('black');
  });

  it('renders correctly with black orientation', () => {
    const { container } = render(
      <Chessboard
        fen="rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
        orientation="black"
      />
    );

    const wrap = container.querySelector('.cg-wrap');
    expect(wrap).toHaveClass('orientation-black');

    const ranksCoords = container.querySelector('coords.ranks');
    expect(ranksCoords).toBeInTheDocument();
    expect(ranksCoords).toHaveClass('black');

    const filesCoords = container.querySelector('coords.files');
    expect(filesCoords).toBeInTheDocument();
    expect(filesCoords).toHaveClass('black');
  });
});
