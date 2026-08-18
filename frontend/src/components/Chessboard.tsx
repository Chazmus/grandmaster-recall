import React, { useEffect, useRef } from 'react';
import { Chessground } from 'chessground';
import { Api } from 'chessground/api';
import { Config } from 'chessground/config';
import { Chess, Square } from 'chess.js';
import { Color } from '../types';
import { sounds } from '../utils/sound';

interface ChessboardProps {
  fen: string;
  orientation: Color;
  turnColor?: Color;
  lastMove?: [string, string];
  shapes?: any[];
  canMove?: boolean;
  onMove?: (orig: string, dest: string, promotion?: string) => void;
  className?: string;
}

export const Chessboard: React.FC<ChessboardProps> = ({
  fen,
  orientation,
  turnColor,
  lastMove,
  shapes = [],
  canMove = true,
  onMove,
  className = '',
}) => {
  const boardRef = useRef<HTMLDivElement>(null);
  const cgApi = useRef<Api | null>(null);

  // Helper to compute legal destinations from chess.js
  const getDests = (chessInstance: Chess) => {
    const dests = new Map<string, string[]>();
    const moves = chessInstance.moves({ verbose: true });
    for (const move of moves) {
      if (!dests.has(move.from)) {
        dests.set(move.from, []);
      }
      dests.get(move.from)!.push(move.to);
    }
    return dests;
  };

  useEffect(() => {
    if (!boardRef.current) return;

    let chessInstance: Chess;
    try {
      chessInstance = new Chess(fen);
    } catch {
      chessInstance = new Chess();
    }

    const currentTurn = (turnColor || (chessInstance.turn() === 'w' ? 'white' : 'black')) as Color;
    const dests = canMove && currentTurn === orientation ? getDests(chessInstance) : new Map();

    const config: Config = {
      fen: fen,
      orientation: orientation,
      turnColor: currentTurn,
      lastMove: lastMove as any,
      coordinates: true,
      movable: {
        free: false,
        color: canMove ? orientation : undefined,
        dests: dests as any,
        showDests: true,
        events: {
          after: (orig, dest) => {
            // Check if it was a capture
            const isCapture = chessInstance.get(dest as Square) !== null;
            if (isCapture) {
              sounds.play('capture');
            } else {
              sounds.play('move');
            }

            // Check if promotion
            let promotion = undefined;
            const piece = chessInstance.get(orig as Square);
            if (
              piece?.type === 'p' &&
              ((dest.endsWith('8') && orientation === 'white') ||
                (dest.endsWith('1') && orientation === 'black'))
            ) {
              promotion = 'q'; // Default to Queen for fast puzzle solving
            }

            if (onMove) {
              onMove(orig, dest, promotion);
            }
          },
        },
      },
      drawable: {
        enabled: true,
        visible: true,
        autoShapes: shapes,
      },
      animation: {
        enabled: true,
        duration: 250,
      },
    };

    if (!cgApi.current) {
      cgApi.current = Chessground(boardRef.current, config);
    } else {
      cgApi.current.set(config);
    }

    return () => {
      // Optional cleanup
    };
  }, [fen, orientation, turnColor, lastMove, shapes, canMove, onMove]);

  return (
    <div className={`relative aspect-square w-full max-w-[560px] mx-auto select-none ${className}`}>
      <div ref={boardRef} className="w-full h-full cg-wrap shadow-2xl rounded-xl border border-slate-700/60" />
    </div>
  );
};
