import React from 'react';
import {
  TrendingUp,
  Brain,
  Award,
  AlertTriangle,
  BookOpen,
  PieChart as PieIcon,
} from 'lucide-react';
import { StatsSummary } from '../types';

interface StatsDashboardProps {
  stats: StatsSummary | null;
  username: string;
}

export const StatsDashboard: React.FC<StatsDashboardProps> = ({ stats, username }) => {
  if (!stats) {
    return (
      <div className="max-w-5xl mx-auto px-4 py-12 text-center text-slate-400">
        Loading analytics...
      </div>
    );
  }

  const totalMistakes = stats.blunders_count + stats.mistakes_count + stats.inaccuracies_count;

  return (
    <div className="max-w-6xl mx-auto px-4 py-8 space-y-8">
      {/* Header */}
      <div>
        <h1 className="text-2xl sm:text-3xl font-extrabold text-white tracking-tight flex items-center gap-2.5">
          <Brain className="w-8 h-8 text-emerald-400" />
          Tactical Analytics & Progress for <span className="text-emerald-400">{username}</span>
        </h1>
        <p className="text-sm text-slate-400 mt-1">
          Detailed breakdown of your blunder patterns, retention rates, and opening weaknesses.
        </p>
      </div>

      {/* Main Stats Grid */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
        {/* Retention & Mastery */}
        <div className="p-6 rounded-3xl bg-slate-900 border border-slate-800 shadow-xl space-y-4">
          <div className="flex items-center justify-between">
            <span className="text-xs font-bold text-slate-400 uppercase tracking-wider">
              Spaced Repetition Retention
            </span>
            <TrendingUp className="w-5 h-5 text-emerald-400" />
          </div>
          <div className="flex items-baseline gap-2">
            <span className="text-4xl font-extrabold text-emerald-400 font-mono">
              {stats.retention_rate.toFixed(1)}%
            </span>
            <span className="text-xs text-slate-400">accuracy on reviews</span>
          </div>
          <div className="w-full bg-slate-800 rounded-full h-2 overflow-hidden">
            <div
              className="bg-emerald-500 h-full rounded-full"
              style={{ width: `${Math.min(100, stats.retention_rate)}%` }}
            />
          </div>
          <div className="text-xs text-slate-400 flex items-center justify-between">
            <span>{stats.mastered_puzzles} Mastered</span>
            <span>{stats.total_reviews} Total reviews</span>
          </div>
        </div>

        {/* Severity Distribution */}
        <div className="p-6 rounded-3xl bg-slate-900 border border-slate-800 shadow-xl space-y-4">
          <div className="flex items-center justify-between">
            <span className="text-xs font-bold text-slate-400 uppercase tracking-wider">
              Mistake Severity
            </span>
            <AlertTriangle className="w-5 h-5 text-amber-400" />
          </div>

          <div className="space-y-2">
            <div className="flex items-center justify-between text-xs">
              <span className="text-rose-400 font-medium flex items-center gap-1.5">
                <span className="w-2 h-2 rounded-full bg-rose-500" /> Blunders (&gt;250 cp)
              </span>
              <span className="font-mono font-bold text-slate-200">{stats.blunders_count}</span>
            </div>
            <div className="flex items-center justify-between text-xs">
              <span className="text-amber-400 font-medium flex items-center gap-1.5">
                <span className="w-2 h-2 rounded-full bg-amber-500" /> Mistakes (120-250 cp)
              </span>
              <span className="font-mono font-bold text-slate-200">{stats.mistakes_count}</span>
            </div>
            <div className="flex items-center justify-between text-xs">
              <span className="text-yellow-400 font-medium flex items-center gap-1.5">
                <span className="w-2 h-2 rounded-full bg-yellow-500" /> Inaccuracies (60-120 cp)
              </span>
              <span className="font-mono font-bold text-slate-200">{stats.inaccuracies_count}</span>
            </div>
          </div>

          <div className="w-full bg-slate-800 rounded-full h-2 flex overflow-hidden">
            {totalMistakes > 0 && (
              <>
                <div
                  className="bg-rose-500 h-full"
                  style={{ width: `${(stats.blunders_count / totalMistakes) * 100}%` }}
                />
                <div
                  className="bg-amber-500 h-full"
                  style={{ width: `${(stats.mistakes_count / totalMistakes) * 100}%` }}
                />
                <div
                  className="bg-yellow-500 h-full"
                  style={{ width: `${(stats.inaccuracies_count / totalMistakes) * 100}%` }}
                />
              </>
            )}
          </div>
        </div>

        {/* Mastered / Queue Balance */}
        <div className="p-6 rounded-3xl bg-slate-900 border border-slate-800 shadow-xl space-y-4">
          <div className="flex items-center justify-between">
            <span className="text-xs font-bold text-slate-400 uppercase tracking-wider">
              Training Pipeline
            </span>
            <Award className="w-5 h-5 text-indigo-400" />
          </div>
          <div className="flex items-baseline gap-2">
            <span className="text-4xl font-extrabold text-white font-mono">
              {stats.total_puzzles}
            </span>
            <span className="text-xs text-slate-400">Total personalized puzzles</span>
          </div>
          <div className="grid grid-cols-2 gap-2 text-xs pt-2">
            <div className="p-2.5 rounded-xl bg-slate-800/60 border border-slate-700/60">
              <span className="text-slate-400">Due Today</span>
              <div className="text-base font-bold text-indigo-400 font-mono mt-0.5">{stats.due_today}</div>
            </div>
            <div className="p-2.5 rounded-xl bg-slate-800/60 border border-slate-700/60">
              <span className="text-slate-400">Mastered</span>
              <div className="text-base font-bold text-emerald-400 font-mono mt-0.5">{stats.mastered_puzzles}</div>
            </div>
          </div>
        </div>
      </div>

      {/* Two Column Section: Tactical Themes vs Opening Blunders */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-8">
        {/* Tactical Themes Breakdown */}
        <div className="p-6 rounded-3xl bg-slate-900 border border-slate-800 shadow-xl space-y-4">
          <div className="flex items-center justify-between">
            <h3 className="text-base font-bold text-slate-100 flex items-center gap-2">
              <PieIcon className="w-5 h-5 text-emerald-400" />
              Tactical Motifs Breakdown
            </h3>
            <span className="text-xs text-slate-500">Frequency & Success Rate</span>
          </div>

          <div className="space-y-3">
            {stats.tactical_tag_breakdown.length > 0 ? (
              stats.tactical_tag_breakdown.map((t, idx) => (
                <div key={idx} className="p-3 rounded-2xl bg-slate-950/60 border border-slate-800 flex items-center justify-between">
                  <div>
                    <span className="font-semibold text-sm text-slate-200">{t.tag}</span>
                    <div className="text-xs text-slate-400 mt-0.5">
                      {t.count} {t.count === 1 ? 'puzzle' : 'puzzles'} generated
                    </div>
                  </div>
                  <div className="text-right">
                    <span className="font-mono text-sm font-bold text-emerald-400">
                      {t.success_rate.toFixed(0)}%
                    </span>
                    <div className="text-[10px] text-slate-500">solve rate</div>
                  </div>
                </div>
              ))
            ) : (
              <div className="text-xs text-slate-500 py-6 text-center">
                No tactical tag data available yet.
              </div>
            )}
          </div>
        </div>

        {/* Weak Openings */}
        <div className="p-6 rounded-3xl bg-slate-900 border border-slate-800 shadow-xl space-y-4">
          <div className="flex items-center justify-between">
            <h3 className="text-base font-bold text-slate-100 flex items-center gap-2">
              <BookOpen className="w-5 h-5 text-amber-400" />
              Top Blundered Openings
            </h3>
            <span className="text-xs text-slate-500">Focus Areas</span>
          </div>

          <div className="space-y-3">
            {stats.top_blundered_openings.length > 0 ? (
              stats.top_blundered_openings.map((op, idx) => (
                <div key={idx} className="p-3 rounded-2xl bg-slate-950/60 border border-slate-800 flex items-center justify-between">
                  <div className="truncate pr-4">
                    <span className="font-semibold text-sm text-slate-200 block truncate">{op.opening_name}</span>
                    <span className="text-xs text-rose-400 font-mono">
                      {op.blunder_count} {op.blunder_count === 1 ? 'blunder' : 'blunders'} recorded
                    </span>
                  </div>
                  <span className="px-2.5 py-1 rounded-lg bg-rose-950/70 border border-rose-800 text-rose-300 text-xs font-bold font-mono shrink-0">
                    Rank #{idx + 1}
                  </span>
                </div>
              ))
            ) : (
              <div className="text-xs text-slate-500 py-6 text-center">
                No opening blunders detected yet.
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};
