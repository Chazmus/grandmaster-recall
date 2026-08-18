import React from 'react';
import { PuzzleWithReview } from '../types';
import { Play } from 'lucide-react';

interface PuzzleListProps {
  puzzles: PuzzleWithReview[];
  onSelectPuzzle: (puzzle: PuzzleWithReview) => void;
  severityFilter: string | null;
  onFilterChange: (severity: string | null) => void;
}

export const PuzzleList: React.FC<PuzzleListProps> = ({
  puzzles,
  onSelectPuzzle,
  severityFilter,
  onFilterChange,
}) => {
  return (
    <div className="max-w-6xl mx-auto px-4 py-8 space-y-6">
      {/* Header & Filter Bar */}
      <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4">
        <div>
          <h1 className="text-2xl font-extrabold text-white">All Generated Puzzles</h1>
          <p className="text-xs text-slate-400 mt-0.5">
            Browse and practice every tactical puzzle extracted from your games.
          </p>
        </div>

        {/* Severity Filters */}
        <div className="flex items-center gap-1.5 p-1 bg-slate-900 border border-slate-800 rounded-xl">
          <button
            onClick={() => onFilterChange(null)}
            className={`px-3 py-1.5 rounded-lg text-xs font-semibold transition-all ${
              severityFilter === null
                ? 'bg-emerald-500 text-slate-950 shadow-md'
                : 'text-slate-400 hover:text-white'
            }`}
          >
            All
          </button>
          <button
            onClick={() => onFilterChange('blunder')}
            className={`px-3 py-1.5 rounded-lg text-xs font-semibold transition-all ${
              severityFilter === 'blunder'
                ? 'bg-rose-500 text-white shadow-md'
                : 'text-slate-400 hover:text-white'
            }`}
          >
            Blunders
          </button>
          <button
            onClick={() => onFilterChange('mistake')}
            className={`px-3 py-1.5 rounded-lg text-xs font-semibold transition-all ${
              severityFilter === 'mistake'
                ? 'bg-amber-500 text-slate-950 shadow-md'
                : 'text-slate-400 hover:text-white'
            }`}
          >
            Mistakes
          </button>
          <button
            onClick={() => onFilterChange('inaccuracy')}
            className={`px-3 py-1.5 rounded-lg text-xs font-semibold transition-all ${
              severityFilter === 'inaccuracy'
                ? 'bg-yellow-500 text-slate-950 shadow-md'
                : 'text-slate-400 hover:text-white'
            }`}
          >
            Inaccuracies
          </button>
        </div>
      </div>

      {/* Grid of Puzzles */}
      {puzzles.length === 0 ? (
        <div className="p-12 text-center bg-slate-900/50 border border-slate-800 rounded-3xl text-slate-500 text-sm">
          No puzzles matching this filter.
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {puzzles.map((pWithRev) => {
            const { puzzle, review, game_white, game_black, game_time_class } = pWithRev;
            const opponent = puzzle.player_color === 'white' ? game_black : game_white;
            const evalDrop = (puzzle.eval_before - puzzle.eval_after_blunder) / 100;

            return (
              <div
                key={puzzle.id}
                onClick={() => onSelectPuzzle(pWithRev)}
                className="cursor-pointer group p-5 rounded-2xl bg-slate-900 border border-slate-800 hover:border-slate-700 hover:bg-slate-850 transition-all duration-200 hover:scale-[1.02] shadow-lg flex flex-col justify-between"
              >
                <div>
                  <div className="flex items-center justify-between mb-2.5">
                    <span className={`px-2 py-0.5 rounded text-[10px] font-bold uppercase tracking-wider ${
                      puzzle.blunder_severity === 'blunder'
                        ? 'bg-rose-950/80 text-rose-300 border border-rose-800/60'
                        : puzzle.blunder_severity === 'mistake'
                        ? 'bg-amber-950/80 text-amber-300 border border-amber-800/60'
                        : 'bg-yellow-950/80 text-yellow-300 border border-yellow-800/60'
                    }`}>
                      {puzzle.blunder_severity}
                    </span>
                    <span className="text-xs text-rose-400 font-mono font-semibold">
                      -{evalDrop.toFixed(1)} eval
                    </span>
                  </div>

                  <div className="font-bold text-slate-100 text-sm group-hover:text-emerald-400 transition-colors">
                    vs {opponent} <span className="text-slate-500 text-xs font-normal">({game_time_class})</span>
                  </div>

                  <div className="text-xs text-slate-400 mt-1">
                    Move {puzzle.move_number} as {puzzle.player_color} • Played <span className="text-rose-400 font-mono font-medium">{puzzle.blunder_move_san}</span>
                  </div>

                  {puzzle.opening_name && (
                    <div className="text-[11px] text-slate-500 mt-2 truncate">
                      {puzzle.opening_name}
                    </div>
                  )}
                </div>

                <div className="pt-3 mt-3 border-t border-slate-800/80 flex items-center justify-between text-xs">
                  <span className="text-slate-500 font-mono">
                    Solved {review.times_solved} / Failed {review.times_failed}
                  </span>
                  <span className="font-semibold text-emerald-400 flex items-center gap-1 group-hover:translate-x-1 transition-transform">
                    Practice <Play className="w-3 h-3 fill-current" />
                  </span>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
};
