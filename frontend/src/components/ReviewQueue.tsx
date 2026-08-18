import React from 'react';
import {
  BrainCircuit,
  Calendar,
  Flame,
  CheckCircle,
  AlertTriangle,
  Play,
  RotateCcw,
  Sparkles,
  TrendingUp,
} from 'lucide-react';
import { PuzzleWithReview, StatsSummary } from '../types';

interface ReviewQueueProps {
  duePuzzles: PuzzleWithReview[];
  stats: StatsSummary | null;
  onStartReview: () => void;
  onSelectPuzzle: (puzzle: PuzzleWithReview) => void;
  onOpenSync: () => void;
  username: string;
}

export const ReviewQueue: React.FC<ReviewQueueProps> = ({
  duePuzzles,
  stats,
  onStartReview,
  onSelectPuzzle,
  onOpenSync,
  username,
}) => {
  const dueCount = stats?.due_today ?? duePuzzles.length;
  const totalCount = stats?.total_puzzles ?? 0;
  const masteredCount = stats?.mastered_puzzles ?? 0;
  const retention = stats?.retention_rate ? stats.retention_rate.toFixed(0) : '100';

  return (
    <div className="max-w-6xl mx-auto px-4 py-8 space-y-8">
      {/* Hero / Daily Review Card */}
      <div className="relative overflow-hidden rounded-3xl bg-gradient-to-r from-slate-900 via-slate-900 to-indigo-950/70 border border-slate-800 p-8 shadow-2xl">
        <div className="relative z-10 flex flex-col md:flex-row items-start md:items-center justify-between gap-6">
          <div>
            <div className="flex items-center gap-2 text-indigo-400 font-semibold text-xs tracking-wider uppercase mb-2">
              <BrainCircuit className="w-4 h-4" />
              Spaced Repetition Active
            </div>
            <h1 className="text-3xl sm:text-4xl font-extrabold text-white tracking-tight">
              Welcome back, <span className="text-emerald-400">{username}</span>
            </h1>
            <p className="text-slate-400 mt-2 text-sm sm:text-base max-w-xl">
              {dueCount > 0
                ? `You have ${dueCount} personal blunder ${dueCount === 1 ? 'puzzle' : 'puzzles'} due for review today.`
                : totalCount > 0
                ? 'All caught up on reviews for today! You can practice ahead or import new games.'
                : 'No puzzles found yet. Connect your Chess.com account to extract blunders and tactical opportunities!'}
            </p>
          </div>

          <div className="flex items-center gap-3 w-full md:w-auto">
            {dueCount > 0 ? (
              <button
                onClick={onStartReview}
                className="flex-1 md:flex-none flex items-center justify-center gap-2.5 px-6 py-3.5 rounded-2xl bg-emerald-500 hover:bg-emerald-400 text-slate-950 font-bold text-base shadow-xl shadow-emerald-950/60 transition-all hover:scale-105 active:scale-95"
              >
                <Play className="w-5 h-5 fill-current" />
                Train {dueCount} Puzzles
              </button>
            ) : totalCount > 0 ? (
              <button
                onClick={onStartReview}
                className="flex-1 md:flex-none flex items-center justify-center gap-2.5 px-6 py-3.5 rounded-2xl bg-slate-800 hover:bg-slate-700 text-white font-semibold text-sm border border-slate-700 transition-colors"
              >
                <RotateCcw className="w-4 h-4" />
                Practice Ahead
              </button>
            ) : (
              <button
                onClick={onOpenSync}
                className="flex-1 md:flex-none flex items-center justify-center gap-2.5 px-6 py-3.5 rounded-2xl bg-emerald-500 hover:bg-emerald-400 text-slate-950 font-bold text-base shadow-xl shadow-emerald-950/60 transition-all hover:scale-105"
              >
                <Sparkles className="w-5 h-5" />
                Import Games Now
              </button>
            )}
          </div>
        </div>
      </div>

      {/* Stats Summary Grid */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
        <div className="p-5 rounded-2xl bg-slate-900/80 border border-slate-800/90 shadow-md">
          <div className="flex items-center justify-between text-slate-400 text-xs font-semibold uppercase tracking-wider mb-2">
            <span>Due Today</span>
            <Calendar className="w-4 h-4 text-indigo-400" />
          </div>
          <div className="text-3xl font-extrabold text-white font-mono">{dueCount}</div>
          <div className="text-xs text-slate-500 mt-1">SM-2 schedule queue</div>
        </div>

        <div className="p-5 rounded-2xl bg-slate-900/80 border border-slate-800/90 shadow-md">
          <div className="flex items-center justify-between text-slate-400 text-xs font-semibold uppercase tracking-wider mb-2">
            <span>Mastered</span>
            <CheckCircle className="w-4 h-4 text-emerald-400" />
          </div>
          <div className="text-3xl font-extrabold text-emerald-400 font-mono">{masteredCount}</div>
          <div className="text-xs text-slate-500 mt-1">Interval &gt;= 21 days</div>
        </div>

        <div className="p-5 rounded-2xl bg-slate-900/80 border border-slate-800/90 shadow-md">
          <div className="flex items-center justify-between text-slate-400 text-xs font-semibold uppercase tracking-wider mb-2">
            <span>Retention Rate</span>
            <TrendingUp className="w-4 h-4 text-blue-400" />
          </div>
          <div className="text-3xl font-extrabold text-blue-400 font-mono">{retention}%</div>
          <div className="text-xs text-slate-500 mt-1">Accuracy on reviews</div>
        </div>

        <div className="p-5 rounded-2xl bg-slate-900/80 border border-slate-800/90 shadow-md">
          <div className="flex items-center justify-between text-slate-400 text-xs font-semibold uppercase tracking-wider mb-2">
            <span>Total Puzzles</span>
            <AlertTriangle className="w-4 h-4 text-amber-400" />
          </div>
          <div className="text-3xl font-extrabold text-amber-400 font-mono">{totalCount}</div>
          <div className="text-xs text-slate-500 mt-1">Generated from games</div>
        </div>
      </div>

      {/* Due Puzzles List */}
      {duePuzzles.length > 0 && (
        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <h2 className="text-lg font-bold text-slate-100 flex items-center gap-2">
              <Flame className="w-5 h-5 text-amber-400" />
              Up Next for Review
            </h2>
            <span className="text-xs text-slate-400 font-mono">
              Showing {duePuzzles.length} puzzles
            </span>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {duePuzzles.map((pWithRev) => {
              const { puzzle, review, game_white, game_black, game_time_class } = pWithRev;
              const opponent = puzzle.player_color === 'white' ? game_black : game_white;
              const tags: string[] = JSON.parse(puzzle.tactical_tags || '[]');

              return (
                <div
                  key={puzzle.id}
                  onClick={() => onSelectPuzzle(pWithRev)}
                  className="group relative cursor-pointer rounded-2xl bg-slate-900/70 hover:bg-slate-850 border border-slate-800 hover:border-slate-700 p-5 transition-all duration-200 hover:scale-[1.02] shadow-lg flex flex-col justify-between"
                >
                  <div>
                    <div className="flex items-center justify-between mb-3">
                      <span className={`px-2 py-0.5 rounded text-[11px] font-bold uppercase tracking-wider ${
                        puzzle.blunder_severity === 'blunder'
                          ? 'bg-rose-950/70 text-rose-300 border border-rose-800/60'
                          : 'bg-amber-950/70 text-amber-300 border border-amber-800/60'
                      }`}>
                        {puzzle.blunder_severity}
                      </span>
                      <span className="text-xs text-slate-500 font-mono">
                        {review.interval_days}d interval
                      </span>
                    </div>

                    <div className="font-semibold text-slate-200 text-sm group-hover:text-emerald-400 transition-colors">
                      vs {opponent} <span className="text-slate-500 text-xs">({game_time_class})</span>
                    </div>

                    <div className="text-xs text-slate-400 mt-1">
                      Move {puzzle.move_number} • You played <span className="text-rose-400 font-mono font-medium">{puzzle.blunder_move_san}</span>
                    </div>

                    {puzzle.opening_name && (
                      <div className="text-[11px] text-slate-500 mt-2 truncate">
                        {puzzle.opening_name}
                      </div>
                    )}
                  </div>

                  <div className="flex items-center justify-between pt-3 mt-3 border-t border-slate-800/80">
                    <div className="flex gap-1 overflow-hidden">
                      {tags.slice(0, 2).map((tag, i) => (
                        <span key={i} className="text-[10px] px-1.5 py-0.5 rounded bg-slate-800 text-slate-400">
                          {tag}
                        </span>
                      ))}
                    </div>
                    <span className="text-xs font-semibold text-emerald-400 flex items-center gap-1 group-hover:translate-x-1 transition-transform">
                      Solve <Play className="w-3 h-3 fill-current" />
                    </span>
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
};
