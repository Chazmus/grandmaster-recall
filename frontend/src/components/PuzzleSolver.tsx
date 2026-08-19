import React, { useState, useEffect } from 'react';
import { Chess, Square } from 'chess.js';
import confetti from 'canvas-confetti';
import {
  Sparkles,
  HelpCircle,
  RotateCcw,
  Eye,
  CheckCircle2,
  ArrowRight,
  TrendingDown,
  Flame,
  Info,
  Swords,
  Loader2,
  Award,
  Undo2,
  Bug,
} from 'lucide-react';

import { PuzzleWithReview } from '../types';
import { Chessboard } from './Chessboard';
import { sounds } from '../utils/sound';
import { api } from '../api/client';

interface PuzzleSolverProps {
  puzzleData: PuzzleWithReview;
  onSolved: (puzzleId: number, success: boolean, quality?: number) => void;
  onNext?: () => void;
  userId: number;
}

interface AlternativeContext {
  fenBefore: string;
  expectedUci: string;
  bestSan: string;
  stepIndex: number;
  remainingMoves: string[];
}

interface BoardSnapshot {
  fen: string;
  lastMove?: [string, string];
  stepIndex: number;
  shapes: any[];
  feedbackMessage: string | null;
  status: 'solving' | 'correct' | 'failed' | 'showing_blunder' | 'showing_best' | 'sandbox';
  isAlternativeSolution: boolean;
  alternativeExplanation: string | null;
  alternativeContext: AlternativeContext | null;
}

export const PuzzleSolver: React.FC<PuzzleSolverProps> = ({
  puzzleData,
  onSolved,
  onNext,
  userId,
}) => {
  const { puzzle, game_white, game_black, game_time_class } = puzzleData;
  const opponent = puzzle.player_color === 'white' ? game_black : game_white;

  // Board state
  const [currentFen, setCurrentFen] = useState<string>(puzzle.initial_fen);
  const [chessInstance, setChessInstance] = useState<Chess>(new Chess(puzzle.initial_fen));
  const [lastMove, setLastMove] = useState<[string, string] | undefined>();
  const [shapes, setShapes] = useState<any[]>([]);

  // Move history stack for step-back / undo navigation
  const [history, setHistory] = useState<BoardSnapshot[]>([]);

  // Solution moves (capped to max 3 plies for concise tactical focus)
  const [solutionMoves, setSolutionMoves] = useState<string[]>([]);
  const [currentStep, setCurrentStep] = useState<number>(0);

  // Status flags
  const [status, setStatus] = useState<'solving' | 'correct' | 'failed' | 'showing_blunder' | 'showing_best' | 'sandbox'>('solving');
  const [hintLevel, setHintLevel] = useState<number>(0);
  const [startTime, setStartTime] = useState<number>(Date.now());
  const [feedbackMessage, setFeedbackMessage] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState<boolean>(false);
  const [isValidatingMove, setIsValidatingMove] = useState<boolean>(false);
  const [srsSaved, setSrsSaved] = useState<boolean>(false);

  // Alternative good-move tracking
  const [isAlternativeSolution, setIsAlternativeSolution] = useState<boolean>(false);
  const [alternativeExplanation, setAlternativeExplanation] = useState<string | null>(null);
  const [alternativeContext, setAlternativeContext] = useState<AlternativeContext | null>(null);
  const [userSolvedFen, setUserSolvedFen] = useState<string | null>(null);
  const [userSolvedLastMove, setUserSolvedLastMove] = useState<[string, string] | undefined>();

  // Parse PV lines & cap to max 3 plies
  useEffect(() => {
    try {
      const parsedCont: string[] = JSON.parse(puzzle.continuation_uci || '[]');
      if (parsedCont.length > 0) {
        // Capped to at most 3 plies (e.g. User move -> Computer reply -> User finish)
        setSolutionMoves(parsedCont.slice(0, 3));
      } else {
        setSolutionMoves([puzzle.best_move_uci]);
      }
    } catch {
      setSolutionMoves([puzzle.best_move_uci]);
    }

    const initialChess = new Chess(puzzle.initial_fen);
    setChessInstance(initialChess);
    setCurrentFen(puzzle.initial_fen);
    setLastMove(undefined);
    setShapes([]);
    setCurrentStep(0);
    setStatus('solving');
    setHintLevel(0);
    setStartTime(Date.now());
    setFeedbackMessage(null);
    setSrsSaved(false);
    setIsAlternativeSolution(false);
    setAlternativeExplanation(null);
    setAlternativeContext(null);
    setUserSolvedFen(null);
    setUserSolvedLastMove(undefined);

    const initialSnapshot: BoardSnapshot = {
      fen: puzzle.initial_fen,
      lastMove: undefined,
      stepIndex: 0,
      shapes: [],
      feedbackMessage: null,
      status: 'solving',
      isAlternativeSolution: false,
      alternativeExplanation: null,
      alternativeContext: null,
    };
    setHistory([initialSnapshot]);
  }, [puzzle.id, puzzle.initial_fen, puzzle.continuation_uci, puzzle.best_move_uci]);

  const parseTags = (): string[] => {
    try {
      return JSON.parse(puzzle.tactical_tags || '[]');
    } catch {
      return ['Tactic'];
    }
  };

  const handleMove = async (orig: string, dest: string, promotion?: string) => {
    if (status === 'sandbox') {
      // In sandbox mode, play move and let engine reply indefinitely
      handleSandboxMove(orig, dest, promotion);
      return;
    }

    if (status !== 'solving' || isValidatingMove) return;

    let actualPromotion = promotion;
    const testChess = new Chess(currentFen);
    const piece = testChess.get(orig as Square);
    if (
      !actualPromotion &&
      piece?.type === 'p' &&
      ((dest.endsWith('8') && piece.color === 'w') || (dest.endsWith('1') && piece.color === 'b'))
    ) {
      actualPromotion = 'q';
    }

    const moveUci = `${orig}${dest}${actualPromotion ? actualPromotion : ''}`;
    const expectedMoveUci = solutionMoves[currentStep] || puzzle.best_move_uci;

    // Check legal in chess.js
    const moveResult = testChess.move({
      from: orig as Square,
      to: dest as Square,
      promotion: actualPromotion,
    });

    if (!moveResult) return;

    // Capture pre-move snapshot to history
    const preMoveSnapshot: BoardSnapshot = {
      fen: currentFen,
      lastMove,
      stepIndex: currentStep,
      shapes,
      feedbackMessage,
      status,
      isAlternativeSolution,
      alternativeExplanation,
      alternativeContext,
    };

    setIsValidatingMove(true);

    try {
      // Validate move against engine (flexible acceptance)
      const valRes = await api.validateMove({
        fen: currentFen,
        move_uci: moveUci,
        expected_best_uci: expectedMoveUci,
        player_color: puzzle.player_color,
      });

      if (valRes.is_valid) {
        setHistory((prev) => [...prev, preMoveSnapshot]);

        if (!valRes.is_best) {
          setIsAlternativeSolution(true);
          if (valRes.explanation) {
            setAlternativeExplanation(valRes.explanation);
          }

          // Use the actual best move for this position returned by the engine, or solutionMoves[currentStep]
          const targetBestUci =
            valRes.best_move_uci ||
            solutionMoves[currentStep] ||
            (currentStep === 0 ? puzzle.best_move_uci : '');

          let stepBestSan = targetBestUci;
          if (targetBestUci && targetBestUci.length >= 4) {
            try {
              const stepChess = new Chess(currentFen);
              const sFrom = targetBestUci.slice(0, 2) as Square;
              const sTo = targetBestUci.slice(2, 4) as Square;
              const sProm = targetBestUci.length > 4 ? targetBestUci[4] : undefined;
              const sRes = stepChess.move({ from: sFrom, to: sTo, promotion: sProm });
              if (sRes) {
                stepBestSan = sRes.san;
              }
            } catch {
              // fallback
            }
          }

          const continuationLine =
            valRes.continuation_uci && valRes.continuation_uci.length > 0
              ? valRes.continuation_uci
              : solutionMoves.slice(currentStep);

          if (targetBestUci) {
            setAlternativeContext({
              fenBefore: currentFen,
              expectedUci: targetBestUci,
              bestSan: stepBestSan,
              stepIndex: currentStep,
              remainingMoves: continuationLine,
            });
          }
        }

        const newFen = testChess.fen();
        setCurrentFen(newFen);
        setChessInstance(testChess);
        setLastMove([orig, dest]);
        setShapes([]);
        setUserSolvedFen(newFen);
        setUserSolvedLastMove([orig, dest]);

        const nextStep = currentStep + 1;
        const maxPlies = Math.min(3, solutionMoves.length);

        // Check if there is an opponent reply to play out
        if (nextStep < maxPlies && valRes.opponent_reply_uci) {
          setFeedbackMessage(valRes.explanation || 'Great move! Continuation...');
          setCurrentStep(nextStep);

          setTimeout(() => {
            const oppMoveUci = valRes.opponent_reply_uci!;
            if (oppMoveUci && oppMoveUci.length >= 4) {
              const oppFrom = oppMoveUci.slice(0, 2);
              const oppTo = oppMoveUci.slice(2, 4);
              const oppProm = oppMoveUci.length > 4 ? oppMoveUci[4] : undefined;

              try {
                const oppChess = new Chess(newFen);
                const oppMoveResult = oppChess.move({
                  from: oppFrom as Square,
                  to: oppTo as Square,
                  promotion: oppProm,
                });

                if (oppMoveResult) {
                  if (oppMoveResult.captured) {
                    sounds.play('capture');
                  } else {
                    sounds.play('move');
                  }
                  const afterOppFen = oppChess.fen();
                  setCurrentFen(afterOppFen);
                  setChessInstance(oppChess);
                  setLastMove([oppFrom, oppTo]);
                  setUserSolvedFen(afterOppFen);
                  setUserSolvedLastMove([oppFrom, oppTo]);
                  setCurrentStep(nextStep + 1);
                  setHintLevel(0);

                  const afterOppSnapshot: BoardSnapshot = {
                    fen: afterOppFen,
                    lastMove: [oppFrom, oppTo],
                    stepIndex: nextStep + 1,
                    shapes: [],
                    feedbackMessage: valRes.explanation || null,
                    status: nextStep + 1 >= maxPlies ? 'correct' : 'solving',
                    isAlternativeSolution: !valRes.is_best,
                    alternativeExplanation: valRes.explanation || null,
                    alternativeContext,
                  };
                  setHistory((prev) => [...prev, afterOppSnapshot]);

                  if (nextStep + 1 >= maxPlies) {
                    finishPuzzleSuccess(valRes.is_best, valRes.explanation);
                  }
                } else {
                  finishPuzzleSuccess(valRes.is_best, valRes.explanation);
                }
              } catch {
                finishPuzzleSuccess(valRes.is_best, valRes.explanation);
              }
            } else {
              finishPuzzleSuccess(valRes.is_best, valRes.explanation);
            }
          }, 500);
        } else {
          finishPuzzleSuccess(valRes.is_best, valRes.explanation);
        }
      } else {
        // Invalid / blunder move
        sounds.play('error');
        setFeedbackMessage(valRes.explanation || 'Not quite. Try a different approach!');
        setStatus('failed');

        setShapes([
          {
            orig: orig,
            dest: dest,
            brush: 'red',
          },
        ]);

        setTimeout(() => {
          setCurrentFen(chessInstance.fen());
          setStatus('solving');
        }, 700);
      }
    } catch (err) {
      console.error('Validation error:', err);
    } finally {
      setIsValidatingMove(false);
    }
  };

  const handleSandboxMove = async (orig: string, dest: string, promotion?: string) => {
    let actualPromotion = promotion;
    const testChess = new Chess(currentFen);
    const piece = testChess.get(orig as Square);
    if (
      !actualPromotion &&
      piece?.type === 'p' &&
      ((dest.endsWith('8') && piece.color === 'w') || (dest.endsWith('1') && piece.color === 'b'))
    ) {
      actualPromotion = 'q';
    }

    try {
      const res = testChess.move({
        from: orig as Square,
        to: dest as Square,
        promotion: actualPromotion,
      });
      if (!res) return;
    } catch {
      return;
    }

    setHistory((prev) => [
      ...prev,
      {
        fen: currentFen,
        lastMove,
        stepIndex: currentStep,
        shapes,
        feedbackMessage,
        status,
        isAlternativeSolution,
        alternativeExplanation,
        alternativeContext,
      },
    ]);

    const newFen = testChess.fen();
    setCurrentFen(newFen);
    setChessInstance(testChess);
    setLastMove([orig, dest]);

    // Engine reply
    try {
      const evalRes = await api.evaluatePosition(newFen, 12, 1);
      if (evalRes.best_move && evalRes.best_move.length >= 4) {
        setTimeout(() => {
          const oppFrom = evalRes.best_move.slice(0, 2);
          const oppTo = evalRes.best_move.slice(2, 4);
          const oppProm = evalRes.best_move.length > 4 ? evalRes.best_move[4] : undefined;

          try {
            const oppChess = new Chess(newFen);
            const oppMoveResult = oppChess.move({
              from: oppFrom as Square,
              to: oppTo as Square,
              promotion: oppProm,
            });

            if (oppMoveResult) {
              if (oppMoveResult.captured) {
                sounds.play('capture');
              } else {
                sounds.play('move');
              }
              setCurrentFen(oppChess.fen());
              setChessInstance(oppChess);
              setLastMove([oppFrom, oppTo]);
            }
          } catch {
            // ignore
          }
        }, 400);
      }
    } catch {
      // ignore
    }
  };

  const finishPuzzleSuccess = (isBest = true, customFeedback?: string) => {
    setStatus('correct');
    sounds.play('victory');
    if (!isBest) {
      setIsAlternativeSolution(true);
    }
    setFeedbackMessage(
      customFeedback ||
        (isBest
          ? 'Brilliant! You found the winning continuation.'
          : alternativeExplanation || 'Good move! You found a sound, winning continuation.')
    );

    try {
      confetti({
        particleCount: 70,
        spread: 60,
        origin: { y: 0.7 },
      });
    } catch {
      // ignore
    }

    if (!srsSaved) {
      setSrsSaved(true);
      onSolved(puzzle.id, true);
    }
  };

  const handleShowBestMove = () => {
    const context = alternativeContext || {
      fenBefore: puzzle.initial_fen,
      expectedUci: puzzle.best_move_uci,
      bestSan: puzzle.best_move_san,
      stepIndex: 0,
      remainingMoves: solutionMoves,
    };

    setStatus('showing_best');
    const demoChess = new Chess(context.fenBefore);

    const bestFrom = context.expectedUci.slice(0, 2);
    const bestTo = context.expectedUci.slice(2, 4);
    const bestProm = context.expectedUci.length > 4 ? context.expectedUci[4] : undefined;

    let moveSuccess = false;
    try {
      const res = demoChess.move({
        from: bestFrom as Square,
        to: bestTo as Square,
        promotion: bestProm,
      });
      if (res) {
        moveSuccess = true;
      }
    } catch (err) {
      console.warn('Could not make move in demo position:', context.expectedUci, err);
    }

    if (!moveSuccess) {
      // Fallback to initial position if context had move mismatch
      try {
        const fallbackChess = new Chess(puzzle.initial_fen);
        const fbFrom = puzzle.best_move_uci.slice(0, 2);
        const fbTo = puzzle.best_move_uci.slice(2, 4);
        const fbProm = puzzle.best_move_uci.length > 4 ? puzzle.best_move_uci[4] : undefined;
        const fbRes = fallbackChess.move({
          from: fbFrom as Square,
          to: fbTo as Square,
          promotion: fbProm,
        });
        if (fbRes) {
          setCurrentFen(fallbackChess.fen());
          setLastMove([fbFrom, fbTo]);
          setShapes([{ orig: fbFrom, dest: fbTo, brush: 'green' }]);
          sounds.play('move');
          setFeedbackMessage(`Stockfish top choice: ${puzzle.best_move_san} (${puzzle.best_move_uci})`);
          return;
        }
      } catch {
        return;
      }
    }

    setCurrentFen(demoChess.fen());
    setLastMove([bestFrom, bestTo]);
    setShapes([{ orig: bestFrom, dest: bestTo, brush: 'green' }]);
    sounds.play('move');
    setFeedbackMessage(
      `Stockfish top choice: ${context.bestSan} (${context.expectedUci})`
    );

    const remaining = context.remainingMoves;
    if (remaining.length > 1) {
      setTimeout(() => {
        try {
          const oppReplyUci = remaining[1];
          if (oppReplyUci && oppReplyUci.length >= 4) {
            const oFrom = oppReplyUci.slice(0, 2);
            const oTo = oppReplyUci.slice(2, 4);
            const oProm = oppReplyUci.length > 4 ? oppReplyUci[4] : undefined;

            const oppRes = demoChess.move({
              from: oFrom as Square,
              to: oTo as Square,
              promotion: oProm,
            });

            if (oppRes) {
              setCurrentFen(demoChess.fen());
              setLastMove([oFrom, oTo]);
              setShapes([
                { orig: bestFrom, dest: bestTo, brush: 'green' },
                { orig: oFrom, dest: oTo, brush: 'yellow' },
              ]);
              if (oppRes.captured) {
                sounds.play('capture');
              } else {
                sounds.play('move');
              }
              setFeedbackMessage(
                `Stockfish line: ${context.bestSan} ... (best continuation)`
              );
            }
          }
        } catch {
          // ignore
        }
      }, 800);
    }
  };

  const handleRestoreUserSolution = () => {
    setStatus('correct');
    if (userSolvedFen) {
      setCurrentFen(userSolvedFen);
      setChessInstance(new Chess(userSolvedFen));
      setLastMove(userSolvedLastMove);
      setShapes([]);
    }
    setFeedbackMessage(
      alternativeExplanation || 'Good move! You found a sound, winning continuation.'
    );
  };

  const handleShowHint = async () => {
    const nextHint = hintLevel + 1;
    setHintLevel(nextHint);

    let targetUci = solutionMoves[currentStep];
    let currentBestSan = '';

    if (targetUci && targetUci.length >= 4) {
      try {
        const testChess = new Chess(currentFen);
        const sFrom = targetUci.slice(0, 2) as Square;
        const sTo = targetUci.slice(2, 4) as Square;
        const sProm = targetUci.length > 4 ? targetUci[4] : undefined;
        const mRes = testChess.move({ from: sFrom, to: sTo, promotion: sProm });
        if (mRes) {
          currentBestSan = mRes.san;
        } else {
          targetUci = undefined as any;
        }
      } catch {
        targetUci = undefined as any;
      }
    }

    if (!targetUci || !currentBestSan) {
      if (currentStep === 0) {
        targetUci = puzzle.best_move_uci;
        currentBestSan = puzzle.best_move_san;
      } else {
        try {
          const evalRes = await api.evaluatePosition(currentFen, 12, 1);
          if (evalRes.best_move && evalRes.best_move.length >= 4) {
            targetUci = evalRes.best_move;
            const testChess = new Chess(currentFen);
            const sFrom = targetUci.slice(0, 2) as Square;
            const sTo = targetUci.slice(2, 4) as Square;
            const sProm = targetUci.length > 4 ? targetUci[4] : undefined;
            const mRes = testChess.move({ from: sFrom, to: sTo, promotion: sProm });
            if (mRes) {
              currentBestSan = mRes.san;
            }
          }
        } catch (err) {
          console.error('Failed to get hint evaluation:', err);
        }
      }
    }

    if (!targetUci || targetUci.length < 4) {
      targetUci = puzzle.best_move_uci;
      currentBestSan = puzzle.best_move_san;
    }

    const fromSq = targetUci.slice(0, 2);
    const toSq = targetUci.slice(2, 4);

    if (nextHint === 1) {
      setShapes([{ orig: fromSq, brush: 'yellow' }]);
      setFeedbackMessage(`Hint: Look at the piece on ${fromSq.toUpperCase()}.`);
    } else if (nextHint === 2) {
      setShapes([{ orig: fromSq, dest: toSq, brush: 'yellow' }]);
      setFeedbackMessage(`Hint: Move from ${fromSq.toUpperCase()} to ${toSq.toUpperCase()}.`);
    } else {
      setShapes([{ orig: fromSq, dest: toSq, brush: 'green' }]);
      setFeedbackMessage(`Best move: ${currentBestSan || targetUci}`);
    }
  };

  const handleShowBlunderPunishment = () => {
    setStatus('showing_blunder');
    const demoChess = new Chess(puzzle.initial_fen);

    const blunderFrom = puzzle.blunder_move_uci.slice(0, 2);
    const blunderTo = puzzle.blunder_move_uci.slice(2, 4);
    const blunderProm = puzzle.blunder_move_uci.length > 4 ? puzzle.blunder_move_uci[4] : undefined;

    try {
      demoChess.move({
        from: blunderFrom as Square,
        to: blunderTo as Square,
        promotion: blunderProm,
      });

      setCurrentFen(demoChess.fen());
      setLastMove([blunderFrom, blunderTo]);
      setShapes([{ orig: blunderFrom, dest: blunderTo, brush: 'red' }]);
      sounds.play('move');
      setFeedbackMessage(`In the game, you played ${puzzle.blunder_move_san}.`);
    } catch {
      return;
    }

    setCurrentFen(demoChess.fen());
    setLastMove([blunderFrom, blunderTo]);
    setShapes([{ orig: blunderFrom, dest: blunderTo, brush: 'red' }]);
    sounds.play('move');
    setFeedbackMessage(`In the game, you played ${puzzle.blunder_move_san}.`);

    setTimeout(() => {
      try {
        const blunderCont: string[] = JSON.parse(puzzle.blunder_continuation_uci || '[]');
        if (blunderCont.length > 0) {
          const punishUci = blunderCont[0];
          const pFrom = punishUci.slice(0, 2);
          const pTo = punishUci.slice(2, 4);
          const pProm = punishUci.length > 4 ? punishUci[4] : undefined;

          demoChess.move({
            from: pFrom as Square,
            to: pTo as Square,
            promotion: pProm,
          });

          setCurrentFen(demoChess.fen());
          setLastMove([pFrom, pTo]);
          setShapes([{ orig: pFrom, dest: pTo, brush: 'red' }]);
          sounds.play('capture');
          setFeedbackMessage(`Opponent could punish with ${punishUci}, seizing total control.`);
        }
      } catch {
        // ignore
      }
    }, 900);
  };

  const handleResetPuzzle = () => {
    const initialChess = new Chess(puzzle.initial_fen);
    setChessInstance(initialChess);
    setCurrentFen(puzzle.initial_fen);
    setLastMove(undefined);
    setShapes([]);
    setCurrentStep(0);
    setStatus('solving');
    setFeedbackMessage(null);
    setIsAlternativeSolution(false);
    setAlternativeExplanation(null);
    setAlternativeContext(null);
    setUserSolvedFen(null);
    setUserSolvedLastMove(undefined);

    const initialSnapshot: BoardSnapshot = {
      fen: puzzle.initial_fen,
      lastMove: undefined,
      stepIndex: 0,
      shapes: [],
      feedbackMessage: null,
      status: 'solving',
      isAlternativeSolution: false,
      alternativeExplanation: null,
      alternativeContext: null,
    };
    setHistory([initialSnapshot]);
  };

  const canStepBack =
    currentStep > 0 ||
    history.length > 1 ||
    status === 'sandbox' ||
    status === 'showing_best' ||
    status === 'showing_blunder' ||
    status === 'correct';

  const handleStepBack = () => {
    if (status === 'showing_best' || status === 'showing_blunder') {
      if (userSolvedFen) {
        handleRestoreUserSolution();
      } else {
        handleResetPuzzle();
      }
      return;
    }

    if (history.length <= 1) {
      handleResetPuzzle();
      return;
    }

    const nextHistory = [...history];
    nextHistory.pop(); // remove current snapshot
    const targetState = nextHistory[nextHistory.length - 1];

    if (!targetState) {
      handleResetPuzzle();
      return;
    }

    setHistory(nextHistory);
    setCurrentFen(targetState.fen);
    setChessInstance(new Chess(targetState.fen));
    setLastMove(targetState.lastMove);
    setCurrentStep(targetState.stepIndex);
    setHintLevel(0);
    setShapes(targetState.shapes);
    setFeedbackMessage(
      targetState.stepIndex === 0
        ? null
        : targetState.feedbackMessage || `Stepped back to Ply ${targetState.stepIndex}.`
    );
    setStatus(targetState.status === 'correct' ? 'solving' : targetState.status);
    setIsAlternativeSolution(targetState.isAlternativeSolution);
    setAlternativeExplanation(targetState.alternativeExplanation);
    setAlternativeContext(targetState.alternativeContext);
    sounds.play('move');
  };

  const handleStartSandbox = () => {
    setStatus('sandbox');
    setFeedbackMessage('Freeplay Mode: Play out any moves against Stockfish!');
  };

  const handleSrsRating = async (quality: number) => {
    setIsSubmitting(true);
    try {
      await api.submitSolve(puzzle.id, {
        user_id: userId,
        success: quality >= 3,
        hints_used: hintLevel,
        time_taken_ms: Date.now() - startTime,
        quality,
      });
      if (onNext) {
        onNext();
      }
    } catch (err) {
      console.error('Failed to submit SRS rating:', err);
    } finally {
      setIsSubmitting(false);
    }
  };

  const evalSwing = (puzzle.eval_before - puzzle.eval_after_blunder) / 100;

  return (
    <div className="grid grid-cols-1 lg:grid-cols-12 gap-8 items-start max-w-7xl mx-auto px-4 py-6">
      {/* Left Column: Board */}
      <div className="lg:col-span-7 flex flex-col items-center">
        {/* Opponent Info Header */}
        <div className="w-full max-w-[560px] flex items-center justify-between px-3 py-2 bg-slate-900/90 border border-slate-800 rounded-t-xl text-xs font-medium text-slate-300">
          <div className="flex items-center gap-2">
            <span className={`w-2.5 h-2.5 rounded-full ${puzzle.player_color === 'white' ? 'bg-slate-700 border border-slate-500' : 'bg-slate-100'}`} />
            <span className="font-semibold text-slate-100">{opponent}</span>
            <span className="text-slate-500 capitalize">({game_time_class})</span>
          </div>
          <div className="flex items-center gap-2">
            {puzzle.opening_name && (
              <span className="px-2 py-0.5 rounded bg-slate-800 text-slate-400 border border-slate-700/60 truncate max-w-[200px]">
                {puzzle.opening_name}
              </span>
            )}
            <span className="text-slate-400">Move {puzzle.move_number}</span>
          </div>
        </div>

        {/* Board Component */}
        <Chessboard
          fen={currentFen}
          orientation={puzzle.player_color}
          lastMove={lastMove}
          shapes={shapes}
          canMove={status === 'solving' || status === 'sandbox'}
          onMove={handleMove}
        />

        {/* User Info Footer */}
        <div className="w-full max-w-[560px] flex items-center justify-between px-3 py-2 bg-slate-900/90 border border-slate-800 rounded-b-xl text-xs font-medium text-slate-300 mt-0">
          <div className="flex items-center gap-2">
            <span className={`w-2.5 h-2.5 rounded-full ${puzzle.player_color === 'white' ? 'bg-slate-100' : 'bg-slate-700 border border-slate-500'}`} />
            <span className="font-semibold text-emerald-400">You</span>
            <span className="text-slate-500">({puzzle.player_color})</span>
          </div>
          <div className="flex items-center gap-2">
            {canStepBack && (
              <button
                onClick={handleStepBack}
                disabled={isValidatingMove}
                title="Go back to previous move / retry"
                className="flex items-center gap-1 px-2.5 py-1 rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-200 hover:text-white text-xs font-medium border border-slate-700 transition-all active:scale-95 shadow-sm"
              >
                <Undo2 className="w-3 h-3 text-blue-400" />
                <span>Undo</span>
              </button>
            )}
            {parseTags().map((tag, i) => (
              <span key={i} className="px-2 py-0.5 rounded-full bg-emerald-950/70 border border-emerald-800/60 text-emerald-300 text-[11px]">
                {tag}
              </span>
            ))}
          </div>
        </div>
      </div>

      {/* Right Column: Puzzle Controls, Blunder Details & Spaced Repetition */}
      <div className="lg:col-span-5 flex flex-col gap-5">
        {/* Blunder Recap Banner */}
        <div className="bg-gradient-to-br from-slate-900 to-slate-950 border border-slate-800 p-5 rounded-2xl shadow-xl">
          <div className="flex items-center justify-between mb-3">
            <div className="flex items-center gap-2">
              <span className={`px-2.5 py-1 rounded-md text-xs font-bold uppercase tracking-wider ${
                puzzle.blunder_severity === 'blunder'
                  ? 'bg-rose-950/80 text-rose-300 border border-rose-800/80'
                  : puzzle.blunder_severity === 'mistake'
                  ? 'bg-amber-950/80 text-amber-300 border border-amber-800/80'
                  : 'bg-yellow-950/80 text-yellow-300 border border-yellow-800/80'
              }`}>
                {puzzle.blunder_severity}
              </span>
              <span className="text-xs text-rose-400 font-mono flex items-center gap-1 font-semibold">
                <TrendingDown className="w-3.5 h-3.5" />
                -{evalSwing.toFixed(1)} eval
              </span>
            </div>
            <span className="text-xs text-slate-400 font-mono">
              Rep #{puzzleData.review.repetition_number}
            </span>
          </div>

          <div className="text-sm text-slate-300 leading-relaxed mb-4">
            In this position you played <span className="font-bold text-rose-400 font-mono text-base px-1.5 py-0.5 rounded bg-rose-950/50 border border-rose-900/60">{puzzle.blunder_move_san}</span>.
            <p className="mt-1 text-slate-400">
              Find any sound winning continuation to maintain the advantage!
            </p>
          </div>

          {/* Feedback Message */}
          {feedbackMessage && (
            <div className={`p-3 rounded-xl text-sm flex items-center gap-2.5 font-medium mb-3 transition-all ${
              status === 'correct'
                ? isAlternativeSolution
                  ? 'bg-amber-950/70 border border-amber-600/50 text-amber-200'
                  : 'bg-emerald-950/80 border border-emerald-700/60 text-emerald-200'
                : status === 'showing_best'
                ? 'bg-amber-950/70 border border-amber-600/60 text-amber-100'
                : status === 'showing_blunder'
                ? 'bg-rose-950/80 border border-rose-700/60 text-rose-200'
                : status === 'sandbox'
                ? 'bg-indigo-950/80 border border-indigo-700/60 text-indigo-200'
                : 'bg-slate-800/90 border border-slate-700 text-slate-200'
            }`}>
              {isValidatingMove ? (
                <Loader2 className="w-5 h-5 text-emerald-400 animate-spin shrink-0" />
              ) : status === 'correct' ? (
                isAlternativeSolution ? (
                  <Sparkles className="w-5 h-5 text-amber-400 shrink-0" />
                ) : (
                  <CheckCircle2 className="w-5 h-5 text-emerald-400 shrink-0" />
                )
              ) : status === 'showing_best' ? (
                <Award className="w-5 h-5 text-amber-400 shrink-0" />
              ) : status === 'showing_blunder' ? (
                <Info className="w-5 h-5 text-rose-400 shrink-0" />
              ) : status === 'sandbox' ? (
                <Swords className="w-5 h-5 text-indigo-400 shrink-0" />
              ) : (
                <HelpCircle className="w-5 h-5 text-amber-400 shrink-0" />
              )}
              <span>{feedbackMessage}</span>
            </div>
          )}

          {/* Action buttons */}
          <div className="flex flex-wrap gap-2 pt-2 border-t border-slate-800/80">
            {/* Step Back / Undo Button */}
            {canStepBack && (
              <button
                onClick={handleStepBack}
                disabled={isValidatingMove}
                title="Step backwards to previous move / ply"
                className="flex-1 min-w-[120px] flex items-center justify-center gap-1.5 px-3 py-2 rounded-xl bg-slate-800 hover:bg-slate-700 text-xs font-semibold text-slate-200 transition-colors disabled:opacity-40 disabled:cursor-not-allowed border border-slate-700/60 shadow-sm"
              >
                <Undo2 className="w-3.5 h-3.5 text-blue-400" />
                <span>{currentStep > 0 ? `Step Back (Ply ${currentStep})` : 'Step Back'}</span>
              </button>
            )}

            <button
              onClick={handleShowHint}
              disabled={status === 'correct' || status === 'showing_best' || hintLevel >= 3}
              className="flex-1 min-w-[120px] flex items-center justify-center gap-1.5 px-3 py-2 rounded-xl bg-slate-800 hover:bg-slate-700 text-xs font-semibold text-slate-200 transition-colors disabled:opacity-50 disabled:cursor-not-allowed border border-slate-700/60"
            >
              <Sparkles className="w-3.5 h-3.5 text-amber-400" />
              {hintLevel === 0 ? 'Hint' : hintLevel === 1 ? 'Show Target' : 'Reveal Move'}
            </button>

            <button
              onClick={handleShowBlunderPunishment}
              className="flex-1 min-w-[150px] flex items-center justify-center gap-1.5 px-3 py-2 rounded-xl bg-rose-950/40 hover:bg-rose-900/60 text-xs font-semibold text-rose-300 transition-colors border border-rose-800/50"
            >
              <Eye className="w-3.5 h-3.5 text-rose-400" />
              Why was my move bad?
            </button>

            {/* Button to view best move when user played a good alternative move */}
            {isAlternativeSolution && status !== 'showing_best' && (
              <button
                onClick={handleShowBestMove}
                className="flex-1 min-w-[150px] flex items-center justify-center gap-1.5 px-3 py-2 rounded-xl bg-amber-500/15 hover:bg-amber-500/25 text-xs font-semibold text-amber-300 transition-all border border-amber-500/40 hover:border-amber-400 shadow-sm"
              >
                <Award className="w-3.5 h-3.5 text-amber-400" />
                <span>See Best Move ({alternativeContext?.bestSan || puzzle.best_move_san})</span>
              </button>
            )}

            {status === 'showing_best' && (
              <button
                onClick={handleRestoreUserSolution}
                className="w-full flex items-center justify-center gap-1.5 px-3 py-2 rounded-xl bg-slate-800 hover:bg-slate-700 text-xs font-semibold text-emerald-300 transition-colors border border-slate-700 shadow-sm"
              >
                <RotateCcw className="w-3.5 h-3.5 text-emerald-400" />
                <span>Back to My Move</span>
              </button>
            )}

            {(status === 'showing_blunder' || status === 'sandbox') && (
              <button
                onClick={handleResetPuzzle}
                className="w-full flex items-center justify-center gap-1.5 px-3 py-2 rounded-xl bg-slate-800 hover:bg-slate-700 text-xs font-semibold text-slate-200 transition-colors border border-slate-700"
              >
                <RotateCcw className="w-3.5 h-3.5" />
                Back to Puzzle
              </button>
            )}
          </div>

          {/* Report an Issue Link */}
          <div className="mt-3 pt-3 border-t border-slate-800/80 flex justify-end">
            <a
              href={`https://github.com/Chazmus/grandmaster-recall/issues/new?title=${encodeURIComponent(`[Puzzle #${puzzle.id}] Issue with puzzle`)}&body=${encodeURIComponent(`### Puzzle Details\n- **Puzzle ID:** ${puzzle.id}\n- **Opening:** ${puzzle.opening_name || 'N/A'}\n- **Move Number:** ${puzzle.move_number}\n- **FEN:** \`${currentFen}\`\n\n### Issue Description\n`)}`}
              target="_blank"
              rel="noopener noreferrer"
              className="inline-flex items-center gap-1.5 text-[11px] text-slate-500 hover:text-rose-400 transition-colors"
            >
              <Bug className="w-3 h-3 text-rose-400/80" />
              <span>Report an issue with this puzzle</span>
            </a>
          </div>
        </div>

        {/* Spaced Repetition Rating Card when solved */}
        {(status === 'correct' || status === 'showing_best') && (
          <div className="bg-slate-900 border border-emerald-800/80 p-5 rounded-2xl shadow-xl animate-fade-in">
            <div className="flex items-center gap-2 mb-2 text-emerald-400 font-semibold text-sm">
              <Flame className="w-4 h-4" />
              Rate your recall (SM-2 Spaced Repetition):
            </div>
            <p className="text-xs text-slate-400 mb-4">
              How easily did you spot the tactic? This tunes your next review schedule.
            </p>

            <div className="grid grid-cols-4 gap-2">
              <button
                onClick={() => handleSrsRating(1)}
                disabled={isSubmitting}
                className="flex flex-col items-center p-2.5 rounded-xl bg-rose-950/40 hover:bg-rose-900/60 border border-rose-800/60 text-xs text-rose-200 transition-all hover:scale-105"
              >
                <span className="font-bold text-sm">Again</span>
                <span className="text-[10px] text-rose-400 mt-1">&lt; 1 day</span>
              </button>

              <button
                onClick={() => handleSrsRating(3)}
                disabled={isSubmitting}
                className="flex flex-col items-center p-2.5 rounded-xl bg-amber-950/40 hover:bg-amber-900/60 border border-amber-800/60 text-xs text-amber-200 transition-all hover:scale-105"
              >
                <span className="font-bold text-sm">Hard</span>
                <span className="text-[10px] text-amber-400 mt-1">1-2 days</span>
              </button>

              <button
                onClick={() => handleSrsRating(4)}
                disabled={isSubmitting}
                className="flex flex-col items-center p-2.5 rounded-xl bg-blue-950/40 hover:bg-blue-900/60 border border-blue-800/60 text-xs text-blue-200 transition-all hover:scale-105"
              >
                <span className="font-bold text-sm">Good</span>
                <span className="text-[10px] text-blue-400 mt-1">3-5 days</span>
              </button>

              <button
                onClick={() => handleSrsRating(5)}
                disabled={isSubmitting}
                className="flex flex-col items-center p-2.5 rounded-xl bg-emerald-950/60 hover:bg-emerald-900/80 border border-emerald-700 text-xs text-emerald-200 transition-all hover:scale-105 font-medium shadow-lg shadow-emerald-950/50"
              >
                <span className="font-bold text-sm">Easy</span>
                <span className="text-[10px] text-emerald-400 mt-1">1+ week</span>
              </button>
            </div>

            <div className="flex items-center gap-3 mt-4">
              <button
                onClick={handleStartSandbox}
                className="flex-1 flex items-center justify-center gap-2 py-3 rounded-xl bg-slate-800 hover:bg-slate-700 text-slate-200 font-semibold text-xs border border-slate-700 transition-colors"
              >
                <Swords className="w-4 h-4 text-amber-400" />
                <span>Freeplay vs Stockfish</span>
              </button>

              {onNext && (
                <button
                  onClick={onNext}
                  className="flex-1 flex items-center justify-center gap-2 py-3 rounded-xl bg-emerald-600 hover:bg-emerald-500 text-white font-semibold text-xs shadow-lg shadow-emerald-900/40 transition-colors"
                >
                  <span>Next Puzzle</span>
                  <ArrowRight className="w-4 h-4" />
                </button>
              )}
            </div>
          </div>
        )}
      </div>
    </div>
  );
};
