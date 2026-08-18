import React, { useState } from 'react';
import {
  Brain,
  Layers,
  BarChart3,
  Sparkles,
  Volume2,
  VolumeX,
} from 'lucide-react';
import { sounds } from '../utils/sound';

interface NavbarProps {
  currentTab: 'review' | 'all' | 'analytics' | 'solver';
  onTabChange: (tab: 'review' | 'all' | 'analytics') => void;
  onOpenSync: () => void;
  username: string;
  onSwitchUser: (username: string) => void;
}

export const Navbar: React.FC<NavbarProps> = ({
  currentTab,
  onTabChange,
  onOpenSync,
  username,
  onSwitchUser,
}) => {
  const [isMuted, setIsMuted] = useState(sounds.isMuted());
  const [showUserMenu, setShowUserMenu] = useState(false);
  const [userInput, setUserInput] = useState('');

  const toggleSound = () => {
    const muted = !sounds.toggleMute();
    setIsMuted(muted);
  };

  const handleUserSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (userInput.trim()) {
      onSwitchUser(userInput.trim());
      setShowUserMenu(false);
      setUserInput('');
    }
  };

  return (
    <nav className="sticky top-0 z-40 w-full bg-slate-950/80 backdrop-blur-md border-b border-slate-800/80">
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 h-16 flex items-center justify-between">
        {/* Brand */}
        <div
          onClick={() => onTabChange('review')}
          className="flex items-center gap-3 cursor-pointer group select-none"
        >
          <div className="w-10 h-10 rounded-xl bg-gradient-to-tr from-emerald-600 to-teal-400 p-0.5 shadow-lg shadow-emerald-950/50 group-hover:scale-105 transition-transform flex items-center justify-center">
            <img src="/piece/cburnett/wN.svg" alt="Knight" className="w-7 h-7 drop-shadow-md" />
          </div>
          <div>
            <span className="text-base font-extrabold text-white tracking-tight block">
              Grandmaster<span className="text-emerald-400">Recall</span>
            </span>
            <span className="text-[10px] text-slate-400 block -mt-1 tracking-wider uppercase font-semibold">
              Spaced Repetition Chess
            </span>
          </div>
        </div>

        {/* Center Tabs */}
        <div className="hidden md:flex items-center gap-1 p-1 bg-slate-900/90 border border-slate-800 rounded-2xl">
          <button
            onClick={() => onTabChange('review')}
            className={`flex items-center gap-2 px-4 py-2 rounded-xl text-xs font-semibold transition-all ${
              currentTab === 'review' || currentTab === 'solver'
                ? 'bg-slate-800 text-emerald-400 shadow-md border border-slate-700/60'
                : 'text-slate-400 hover:text-slate-200'
            }`}
          >
            <Brain className="w-4 h-4" />
            Review Queue
          </button>

          <button
            onClick={() => onTabChange('all')}
            className={`flex items-center gap-2 px-4 py-2 rounded-xl text-xs font-semibold transition-all ${
              currentTab === 'all'
                ? 'bg-slate-800 text-emerald-400 shadow-md border border-slate-700/60'
                : 'text-slate-400 hover:text-slate-200'
            }`}
          >
            <Layers className="w-4 h-4" />
            All Puzzles
          </button>

          <button
            onClick={() => onTabChange('analytics')}
            className={`flex items-center gap-2 px-4 py-2 rounded-xl text-xs font-semibold transition-all ${
              currentTab === 'analytics'
                ? 'bg-slate-800 text-emerald-400 shadow-md border border-slate-700/60'
                : 'text-slate-400 hover:text-slate-200'
            }`}
          >
            <BarChart3 className="w-4 h-4" />
            Analytics
          </button>
        </div>

        {/* Right Actions */}
        <div className="flex items-center gap-3">
          {/* Sound Toggle */}
          <button
            onClick={toggleSound}
            title={isMuted ? 'Unmute chess sounds' : 'Mute chess sounds'}
            className="p-2 rounded-xl text-slate-400 hover:text-slate-200 hover:bg-slate-900 border border-slate-800/80 transition-colors"
          >
            {isMuted ? <VolumeX className="w-4 h-4" /> : <Volume2 className="w-4 h-4 text-emerald-400" />}
          </button>

          {/* Import Games Button */}
          <button
            onClick={onOpenSync}
            className="hidden sm:flex items-center gap-2 px-3.5 py-2 rounded-xl bg-slate-900 hover:bg-slate-850 text-slate-200 hover:text-white border border-slate-800 hover:border-slate-700 text-xs font-semibold transition-all shadow-md"
          >
            <Sparkles className="w-3.5 h-3.5 text-amber-400" />
            Sync Games
          </button>

          {/* User Profile / Switcher Dropdown */}
          <div className="relative">
            <button
              onClick={() => setShowUserMenu(!showUserMenu)}
              className="flex items-center gap-2 px-3 py-1.5 rounded-xl bg-slate-900 border border-slate-800 hover:border-slate-700 text-xs font-semibold text-slate-200 transition-colors"
            >
              <div className="w-5 h-5 rounded-full bg-emerald-950 border border-emerald-700 text-emerald-300 flex items-center justify-center text-[10px] font-bold">
                {username.slice(0, 1).toUpperCase()}
              </div>
              <span className="max-w-[90px] truncate">{username}</span>
            </button>

            {showUserMenu && (
              <div className="absolute right-0 mt-2 w-64 p-4 rounded-2xl bg-slate-900 border border-slate-800 shadow-2xl z-50 animate-fade-in">
                <span className="text-xs font-bold text-slate-300 block mb-2">Switch Chess.com Account</span>
                <form onSubmit={handleUserSubmit} className="space-y-2">
                  <input
                    type="text"
                    placeholder="Enter username"
                    value={userInput}
                    onChange={(e) => setUserInput(e.target.value)}
                    className="w-full px-3 py-1.5 rounded-lg bg-slate-950 border border-slate-700 text-xs text-slate-100 focus:outline-none focus:border-emerald-500"
                    autoFocus
                  />
                  <button
                    type="submit"
                    className="w-full py-1.5 rounded-lg bg-emerald-500 hover:bg-emerald-400 text-slate-950 font-bold text-xs transition-colors"
                  >
                    Switch Player
                  </button>
                </form>
              </div>
            )}
          </div>
        </div>
      </div>
    </nav>
  );
};
