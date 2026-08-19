import React, { useState, useEffect, useRef } from 'react';
import { X, Sparkles, Loader2, CheckCircle2, AlertCircle, RefreshCw } from 'lucide-react';
import { SyncStatus } from '../types';
import { api } from '../api/client';

interface GameSyncModalProps {
  isOpen: boolean;
  onClose: () => void;
  currentUsername: string;
  onSyncComplete: (username: string) => void;
}

export const GameSyncModal: React.FC<GameSyncModalProps> = ({
  isOpen,
  onClose,
  currentUsername,
  onSyncComplete,
}) => {
  const [username, setUsername] = useState(currentUsername || 'hikaru');
  const [timeControls, setTimeControls] = useState<{ [key: string]: boolean }>({
    rapid: true,
    blitz: true,
    bullet: false,
    daily: false,
  });
  const [isSyncing, setIsSyncing] = useState<boolean>(false);
  const [syncStatus, setSyncStatus] = useState<SyncStatus | null>(null);
  const [error, setError] = useState<string | null>(null);

  const pollTimer = useRef<any>(null);

  useEffect(() => {
    if (currentUsername) {
      setUsername(currentUsername);
    }
  }, [currentUsername]);

  useEffect(() => {
    return () => {
      if (pollTimer.current) clearInterval(pollTimer.current);
    };
  }, []);

  if (!isOpen) return null;

  const handleStartSync = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!username.trim()) return;

    setError(null);
    setIsSyncing(true);

    const selectedTimeClasses = Object.keys(timeControls).filter((k) => timeControls[k]);

    try {
      const status = await api.startSync({
        username: username.trim(),
        time_classes: selectedTimeClasses,
        max_games: 5,
        months_back: 2,
        engine_depth: 16,
      });

      setSyncStatus(status);

      // Start polling for status
      if (pollTimer.current) clearInterval(pollTimer.current);
      pollTimer.current = setInterval(async () => {
        try {
          const current = await api.getSyncStatus(username.trim());
          setSyncStatus(current);

          if (current.state === 'completed') {
            setIsSyncing(false);
            clearInterval(pollTimer.current);
            onSyncComplete(username.trim());
          } else if (current.state === 'failed') {
            setIsSyncing(false);
            setError(current.error || 'Failed to sync games');
            clearInterval(pollTimer.current);
          }
        } catch {
          // ignore
        }
      }, 1000);
    } catch (err: any) {
      setIsSyncing(false);
      setError(err.message || 'Failed to start sync');
    }
  };

  const progressPercent =
    syncStatus && syncStatus.total_games > 0
      ? Math.min(100, Math.round((syncStatus.processed_games / syncStatus.total_games) * 100))
      : 0;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/80 backdrop-blur-sm animate-fade-in">
      <div className="relative w-full max-w-lg bg-slate-900 border border-slate-800 rounded-3xl p-6 sm:p-8 shadow-2xl overflow-hidden">
        {/* Header */}
        <div className="flex items-center justify-between pb-4 border-b border-slate-800">
          <div className="flex items-center gap-2.5">
            <div className="p-2 rounded-xl bg-emerald-950/70 border border-emerald-800/80 text-emerald-400">
              <Sparkles className="w-5 h-5" />
            </div>
            <div>
              <h2 className="text-lg font-bold text-slate-100">Import & Analyze Games</h2>
              <p className="text-xs text-slate-400">Fetch Chess.com archives & extract tactical blunders</p>
            </div>
          </div>
          <button
            onClick={onClose}
            disabled={isSyncing}
            className="p-1.5 rounded-lg text-slate-400 hover:text-white hover:bg-slate-800 transition-colors disabled:opacity-30"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {error && (
          <div className="mt-4 p-3 rounded-xl bg-rose-950/80 border border-rose-800/80 text-rose-200 text-xs flex items-center gap-2">
            <AlertCircle className="w-4 h-4 shrink-0 text-rose-400" />
            <span>{error}</span>
          </div>
        )}

        {/* Sync in Progress UI */}
        {isSyncing ? (
          <div className="py-8 text-center space-y-5">
            <div className="relative inline-flex items-center justify-center">
              <Loader2 className="w-12 h-12 text-emerald-400 animate-spin" />
            </div>

            <div>
              <h3 className="text-base font-semibold text-slate-100">
                {syncStatus?.state === 'fetching_games'
                  ? 'Fetching Chess.com archives...'
                  : 'Analyzing games with Stockfish...'}
              </h3>
              <p className="text-xs text-slate-400 mt-1 font-mono">
                {syncStatus?.current_game || 'Preparing games for tactical evaluation...'}
              </p>
            </div>

            {/* Progress Bar */}
            <div className="w-full bg-slate-800 rounded-full h-3 overflow-hidden border border-slate-700">
              <div
                className="bg-emerald-500 h-full rounded-full transition-all duration-300 ease-out"
                style={{ width: `${progressPercent}%` }}
              />
            </div>

            <div className="flex items-center justify-between text-xs text-slate-400 font-mono">
              <span>{syncStatus?.processed_games || 0} / {syncStatus?.total_games || 0} games analyzed</span>
              <span className="text-emerald-400 font-bold">{syncStatus?.puzzles_found || 0} blunders extracted</span>
            </div>
          </div>
        ) : syncStatus?.state === 'completed' ? (
          <div className="py-8 text-center space-y-4">
            <div className="p-3 bg-emerald-950/80 border border-emerald-700/80 rounded-full w-14 h-14 mx-auto flex items-center justify-center text-emerald-400">
              <CheckCircle2 className="w-8 h-8" />
            </div>
            <h3 className="text-lg font-bold text-slate-100">Analysis Complete!</h3>
            <p className="text-sm text-slate-300">
              Extracted <span className="font-bold text-emerald-400 font-mono text-base">{syncStatus.puzzles_found}</span> new personalized blunder puzzles for <span className="font-semibold text-white">{username}</span>.
            </p>
            <button
              onClick={() => {
                setSyncStatus(null);
                onClose();
              }}
              className="w-full py-3 rounded-xl bg-emerald-500 hover:bg-emerald-400 text-slate-950 font-bold text-sm transition-all mt-4"
            >
              Start Solving Puzzles
            </button>
          </div>
        ) : (
          <form onSubmit={handleStartSync} className="mt-5 space-y-5">
            {/* Username Input */}
            <div>
              <label className="block text-xs font-semibold text-slate-300 uppercase tracking-wider mb-1.5">
                Chess.com Username
              </label>
              <input
                type="text"
                value={username}
                onChange={(e) => setUsername(e.target.value)}
                placeholder="e.g. MagnusCarlsen, DanielNaroditsky"
                className="w-full px-4 py-2.5 rounded-xl bg-slate-950 border border-slate-700 text-slate-100 text-sm focus:outline-none focus:border-emerald-500 transition-colors font-medium"
                required
              />
            </div>

            {/* Time Controls */}
            <div>
              <label className="block text-xs font-semibold text-slate-300 uppercase tracking-wider mb-2">
                Time Controls
              </label>
              <div className="grid grid-cols-2 sm:grid-cols-4 gap-2">
                {(['rapid', 'blitz', 'bullet', 'daily'] as const).map((tc) => (
                  <label
                    key={tc}
                    className={`flex items-center justify-center gap-2 p-2.5 rounded-xl border text-xs font-semibold capitalize cursor-pointer transition-all ${
                      timeControls[tc]
                        ? 'bg-emerald-950/60 border-emerald-700 text-emerald-300 shadow-sm'
                        : 'bg-slate-950 border-slate-800 text-slate-400 hover:bg-slate-800'
                    }`}
                  >
                    <input
                      type="checkbox"
                      checked={timeControls[tc]}
                      onChange={(e) =>
                        setTimeControls((prev) => ({ ...prev, [tc]: e.target.checked }))
                      }
                      className="hidden"
                    />
                    <span>{tc}</span>
                  </label>
                ))}
              </div>
            </div>

            {/* Background Daemon Info Banner */}
            <div className="p-3.5 rounded-2xl bg-slate-950/80 border border-slate-800 text-xs text-slate-400 space-y-1">
              <div className="flex items-center gap-2 text-emerald-400 font-semibold">
                <Sparkles className="w-3.5 h-3.5" />
                <span>Automatic Background Buffer Active</span>
              </div>
              <p className="text-[11px] leading-relaxed text-slate-400">
                The server automatically maintains 10–16 fresh blunder puzzles at <strong>Depth 16</strong> in the background without pegging your CPU.
              </p>
            </div>

            {/* Action button */}
            <button
              type="submit"
              className="w-full py-3.5 rounded-2xl bg-emerald-500 hover:bg-emerald-400 text-slate-950 font-bold text-sm shadow-xl shadow-emerald-950/60 transition-all hover:scale-[1.01] active:scale-98 flex items-center justify-center gap-2"
            >
              <RefreshCw className="w-4 h-4" />
              Connect & Start Training
            </button>
          </form>
        )}
      </div>
    </div>
  );
};
