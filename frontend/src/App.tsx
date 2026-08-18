import { useState, useEffect } from 'react';
import { Navbar } from './components/Navbar';
import { ReviewQueue } from './components/ReviewQueue';
import { PuzzleSolver } from './components/PuzzleSolver';
import { PuzzleList } from './components/PuzzleList';
import { StatsDashboard } from './components/StatsDashboard';
import { GameSyncModal } from './components/GameSyncModal';
import { PuzzleWithReview, StatsSummary, User } from './types';
import { api } from './api/client';
import { ArrowLeft, Loader2 } from 'lucide-react';

export function App() {
  const [username, setUsername] = useState<string>(() => {
    return localStorage.getItem('chess_trainer_username') || 'hikaru';
  });
  const [user, setUser] = useState<User | null>(null);

  const [activeTab, setActiveTab] = useState<'review' | 'all' | 'analytics' | 'solver'>('review');
  const [isSyncModalOpen, setIsSyncModalOpen] = useState<boolean>(false);

  // Puzzle state
  const [duePuzzles, setDuePuzzles] = useState<PuzzleWithReview[]>([]);
  const [allPuzzles, setAllPuzzles] = useState<PuzzleWithReview[]>([]);
  const [currentPuzzle, setCurrentPuzzle] = useState<PuzzleWithReview | null>(null);
  const [puzzleQueueIndex, setPuzzleQueueIndex] = useState<number>(0);
  const [severityFilter, setSeverityFilter] = useState<string | null>(null);

  // Stats state
  const [stats, setStats] = useState<StatsSummary | null>(null);
  const [isLoading, setIsLoading] = useState<boolean>(true);

  // Initialize or fetch user
  useEffect(() => {
    localStorage.setItem('chess_trainer_username', username);
    loadUserData(username);
  }, [username]);

  const loadUserData = async (uname: string) => {
    setIsLoading(true);
    try {
      const u = await api.getOrCreateUser(uname);
      setUser(u);

      const [due, all, s] = await Promise.all([
        api.getReviewQueue(u.id, 30),
        api.getAllPuzzles(u.id, 60, 0, severityFilter || undefined),
        api.getStats(u.id),
      ]);

      setDuePuzzles(due);
      setAllPuzzles(all);
      setStats(s);

      // If no puzzles exist yet, suggest opening the sync modal
      if (s.total_puzzles === 0) {
        setIsSyncModalOpen(true);
      }
    } catch (err) {
      console.error('Failed to load user data:', err);
    } finally {
      setIsLoading(false);
    }
  };

  const handleFilterChange = async (severity: string | null) => {
    setSeverityFilter(severity);
    if (!user) return;
    try {
      const all = await api.getAllPuzzles(user.id, 60, 0, severity || undefined);
      setAllPuzzles(all);
    } catch (err) {
      console.error('Failed to filter puzzles:', err);
    }
  };

  const handleStartReview = () => {
    if (duePuzzles.length > 0) {
      setCurrentPuzzle(duePuzzles[0]);
      setPuzzleQueueIndex(0);
      setActiveTab('solver');
    } else if (allPuzzles.length > 0) {
      setCurrentPuzzle(allPuzzles[0]);
      setPuzzleQueueIndex(0);
      setActiveTab('solver');
    }
  };

  const handleSelectPuzzle = (pWithRev: PuzzleWithReview) => {
    setCurrentPuzzle(pWithRev);
    setActiveTab('solver');
  };

  const handlePuzzleSolved = async (_puzzleId: number, _success: boolean) => {
    if (!user) return;
    // Refresh stats in background
    api.getStats(user.id).then(setStats).catch(console.error);
  };

  const handleNextPuzzle = () => {
    const nextIdx = puzzleQueueIndex + 1;
    if (nextIdx < duePuzzles.length) {
      setPuzzleQueueIndex(nextIdx);
      setCurrentPuzzle(duePuzzles[nextIdx]);
    } else if (nextIdx < allPuzzles.length) {
      setPuzzleQueueIndex(nextIdx);
      setCurrentPuzzle(allPuzzles[nextIdx]);
    } else {
      // Completed review queue
      setActiveTab('review');
      if (user) {
        loadUserData(user.username);
      }
    }
  };

  const handleSwitchUser = (newUsername: string) => {
    setUsername(newUsername);
    setActiveTab('review');
  };

  const handleSyncComplete = (syncedUsername: string) => {
    loadUserData(syncedUsername);
  };

  return (
    <div className="min-h-screen bg-slate-950 text-slate-100 flex flex-col selection:bg-emerald-500/30 selection:text-emerald-200">
      <Navbar
        currentTab={activeTab}
        onTabChange={(tab) => setActiveTab(tab)}
        onOpenSync={() => setIsSyncModalOpen(true)}
        username={username}
        onSwitchUser={handleSwitchUser}
      />

      <main className="flex-1 pb-16">
        {isLoading ? (
          <div className="flex flex-col items-center justify-center min-h-[60vh] gap-3 text-slate-400">
            <Loader2 className="w-8 h-8 animate-spin text-emerald-400" />
            <p className="text-sm">Connecting to chess engine & database...</p>
          </div>
        ) : (
          <>
            {activeTab === 'review' && (
              <ReviewQueue
                duePuzzles={duePuzzles}
                stats={stats}
                onStartReview={handleStartReview}
                onSelectPuzzle={handleSelectPuzzle}
                onOpenSync={() => setIsSyncModalOpen(true)}
                username={username}
              />
            )}

            {activeTab === 'solver' && currentPuzzle && user && (
              <div className="space-y-4">
                <div className="max-w-7xl mx-auto px-4 pt-4">
                  <button
                    onClick={() => setActiveTab('review')}
                    className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-xl bg-slate-900 hover:bg-slate-850 text-xs font-semibold text-slate-300 border border-slate-800 transition-colors"
                  >
                    <ArrowLeft className="w-4 h-4" />
                    Back to Queue
                  </button>
                </div>
                <PuzzleSolver
                  puzzleData={currentPuzzle}
                  onSolved={handlePuzzleSolved}
                  onNext={handleNextPuzzle}
                  userId={user.id}
                />
              </div>
            )}

            {activeTab === 'all' && (
              <PuzzleList
                puzzles={allPuzzles}
                onSelectPuzzle={handleSelectPuzzle}
                severityFilter={severityFilter}
                onFilterChange={handleFilterChange}
              />
            )}

            {activeTab === 'analytics' && (
              <StatsDashboard stats={stats} username={username} />
            )}
          </>
        )}
      </main>

      <GameSyncModal
        isOpen={isSyncModalOpen}
        onClose={() => setIsSyncModalOpen(false)}
        currentUsername={username}
        onSyncComplete={handleSyncComplete}
      />
    </div>
  );
}
export default App;
