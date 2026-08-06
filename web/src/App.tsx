import { useCallback, useEffect, useRef, useState } from 'react'
import './App.css'
import { ArmPanel, type ArmHighlight } from './components/ArmPanel'
import { BOARD_HEIGHT, type BoardHandle } from './components/Board'
import { PiecePreview } from './components/PiecePreview'
import { SelectionTimer, type SelectionTimerHandle } from './components/SelectionTimer'
import { Hud } from './components/Hud'
import { Controls } from './components/Controls'
import { Credits } from './components/Credits'
import { useGameLoop } from './game/useGameLoop'
import { useKeyboardInput } from './game/useKeyboardInput'
import { useGamepadInput } from './game/useGamepadInput'
import { effects } from './game/effects'
import { initWasm, WasmArm, WasmCrossGame } from './wasm'

const OUTER_PADDING = 16
const GRID_GAP = 8
// Approximate vertical overhead per stacked well row (label + gap + panel
// padding) — used only to size cells to fill the viewport, so it doesn't
// need to be exact, just conservative enough not to overflow.
const PANEL_CHROME_PER_ROW = 45

// N, the W/E row, and S stack to 3 well-heights tall; size cells so that
// stack fills the viewport height instead of leaving unused space below.
function computeCellSize(): number {
  if (typeof window === 'undefined') return 16
  const chrome = OUTER_PADDING * 2 + GRID_GAP * 2 + PANEL_CHROME_PER_ROW * 3
  const available = window.innerHeight - chrome
  const size = Math.floor(available / (BOARD_HEIGHT * 3))
  return Math.max(8, Math.min(34, size))
}

const ARMS = [
  { arm: WasmArm.North, label: 'N' },
  { arm: WasmArm.East, label: 'E' },
  { arm: WasmArm.South, label: 'S' },
  { arm: WasmArm.West, label: 'W' },
] as const

type ArmHud = { score: number; gameOver: boolean; selectable: boolean }
const EMPTY_ARM_HUD: ArmHud = { score: 0, gameOver: false, selectable: true }
const EMPTY_ARM_HUD_MAP: Record<WasmArm, ArmHud> = {
  [WasmArm.North]: EMPTY_ARM_HUD,
  [WasmArm.East]: EMPTY_ARM_HUD,
  [WasmArm.South]: EMPTY_ARM_HUD,
  [WasmArm.West]: EMPTY_ARM_HUD,
}

function randomSeed(): bigint {
  return BigInt(Math.floor(Math.random() * Number.MAX_SAFE_INTEGER))
}

export default function App() {
  const [ready, setReady] = useState(false)
  const [aiEnabled, setAiEnabled] = useState(false)
  const [totalScore, setTotalScore] = useState(0)
  const [gameOver, setGameOver] = useState(false)
  const [awaitingSelection, setAwaitingSelection] = useState(true)
  const [activeArm, setActiveArm] = useState<WasmArm | null>(null)
  const [nextPieceKind, setNextPieceKind] = useState(0)
  const [armHud, setArmHud] = useState<Record<WasmArm, ArmHud>>(EMPTY_ARM_HUD_MAP)
  const [cellSize, setCellSize] = useState(computeCellSize)

  useEffect(() => {
    const onResize = () => setCellSize(computeCellSize())
    window.addEventListener('resize', onResize)
    return () => window.removeEventListener('resize', onResize)
  }, [])

  const gameRef = useRef<WasmCrossGame | null>(null)
  const selectionTimerRef = useRef<SelectionTimerHandle>(null)
  const boardRefs = {
    [WasmArm.North]: useRef<BoardHandle>(null),
    [WasmArm.East]: useRef<BoardHandle>(null),
    [WasmArm.South]: useRef<BoardHandle>(null),
    [WasmArm.West]: useRef<BoardHandle>(null),
  }
  const hudRef = useRef({ totalScore, gameOver, awaitingSelection, activeArm, nextPieceKind, armHud })
  // Tracked separately from display state: effect triggers must fire on the
  // true frame-to-frame diff, not just on the (coalesced) React state update.
  const effectStateRef = useRef({ totalPiecesPlaced: 0, totalLinesCleared: 0, gameOver: false, activeArm: null as WasmArm | null })

  const startNewGame = useCallback(() => {
    gameRef.current?.free()
    gameRef.current = new WasmCrossGame(randomSeed())
    setTotalScore(0)
    setGameOver(false)
    setAwaitingSelection(true)
    setActiveArm(null)
    setNextPieceKind(0)
    setArmHud(EMPTY_ARM_HUD_MAP)
    effectStateRef.current = { totalPiecesPlaced: 0, totalLinesCleared: 0, gameOver: false, activeArm: null }
  }, [])

  useEffect(() => {
    let cancelled = false
    initWasm().then(() => {
      if (cancelled) return
      startNewGame()
      setReady(true)
    })
    return () => {
      cancelled = true
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  const onFrame = useCallback(() => {
    const game = gameRef.current
    if (!game) return

    for (const { arm } of ARMS) {
      boardRefs[arm].current?.draw(game.board_buffer(arm))
    }

    const awaitingNow = game.awaiting_well_selection()
    const remainingFraction = awaitingNow ? 1 - game.selection_timer_ms() / game.selection_timeout_ms() : 1
    selectionTimerRef.current?.setFraction(remainingFraction)

    const nextArmHud = Object.fromEntries(
      ARMS.map(({ arm }) => [
        arm,
        { score: game.score(arm), gameOver: game.arm_game_over(arm), selectable: game.is_well_selectable(arm) },
      ]),
    ) as Record<WasmArm, ArmHud>
    const rawActiveArm = game.active_arm()
    const next = {
      totalScore: game.total_score(),
      gameOver: game.is_game_over(),
      awaitingSelection: awaitingNow,
      activeArm: rawActiveArm >= 0 ? (rawActiveArm as WasmArm) : null,
      nextPieceKind: game.next_queue(1)[0] ?? 0,
      armHud: nextArmHud,
    }

    // Fire audio/haptic feedback from the raw diff, independent of whether
    // display state below happens to coalesce this frame.
    const totalPiecesPlaced = game.total_pieces_placed()
    const totalLinesCleared = game.total_lines_cleared()
    const es = effectStateRef.current
    if (totalPiecesPlaced > es.totalPiecesPlaced) effects.lock()
    if (totalLinesCleared > es.totalLinesCleared) effects.lineClear(totalLinesCleared - es.totalLinesCleared)
    if (next.gameOver && !es.gameOver) effects.gameOver()
    // A piece became active without an AI step in between (manual or
    // auto-timeout selection) — AI's atomic select+drop never leaves a
    // frame where activeArm is observable, so this only fires for human play.
    if (next.activeArm !== null && es.activeArm === null && !aiEnabled) effects.select()
    effectStateRef.current = {
      totalPiecesPlaced,
      totalLinesCleared,
      gameOver: next.gameOver,
      activeArm: next.activeArm,
    }

    const prev = hudRef.current
    const armHudChanged = ARMS.some(
      ({ arm }) =>
        prev.armHud[arm].score !== next.armHud[arm].score ||
        prev.armHud[arm].gameOver !== next.armHud[arm].gameOver ||
        prev.armHud[arm].selectable !== next.armHud[arm].selectable,
    )
    if (
      next.totalScore !== prev.totalScore ||
      next.gameOver !== prev.gameOver ||
      next.awaitingSelection !== prev.awaitingSelection ||
      next.activeArm !== prev.activeArm ||
      next.nextPieceKind !== prev.nextPieceKind ||
      armHudChanged
    ) {
      hudRef.current = next
      setTotalScore(next.totalScore)
      setGameOver(next.gameOver)
      setAwaitingSelection(next.awaitingSelection)
      setActiveArm(next.activeArm)
      setNextPieceKind(next.nextPieceKind)
      setArmHud(next.armHud)
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [aiEnabled])

  useGameLoop(gameRef, aiEnabled, onFrame)
  useKeyboardInput(gameRef, ready && !aiEnabled)
  useGamepadInput(gameRef, ready && !aiEnabled)

  const highlightFor = (arm: WasmArm): ArmHighlight => {
    if (activeArm === arm) return 'active'
    if (awaitingSelection && !aiEnabled) {
      return armHud[arm].selectable ? 'selectable' : 'blocked'
    }
    return 'none'
  }

  return (
    <div
      style={{
        display: 'flex',
        gap: 24,
        padding: OUTER_PADDING,
        alignItems: 'center',
        justifyContent: 'center',
        minHeight: '100dvh',
        boxSizing: 'border-box',
      }}
    >
      <div
        style={{
          display: 'grid',
          gridTemplateColumns: 'repeat(3, auto)',
          gridTemplateRows: 'repeat(3, auto)',
          gap: GRID_GAP,
          justifyItems: 'center',
          alignItems: 'center',
        }}
      >
        <div style={{ gridColumn: 2, gridRow: 1 }}>
          <ArmPanel
            ref={boardRefs[WasmArm.North]}
            label="N"
            cellSize={cellSize}
            score={armHud[WasmArm.North].score}
            gameOver={armHud[WasmArm.North].gameOver}
            highlight={highlightFor(WasmArm.North)}
          />
        </div>
        <div style={{ gridColumn: 1, gridRow: 2 }}>
          <ArmPanel
            ref={boardRefs[WasmArm.West]}
            label="W"
            cellSize={cellSize}
            score={armHud[WasmArm.West].score}
            gameOver={armHud[WasmArm.West].gameOver}
            highlight={highlightFor(WasmArm.West)}
          />
        </div>
        <div
          style={{
            gridColumn: 2,
            gridRow: 2,
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            gap: 2,
            opacity: awaitingSelection ? 1 : 0.35,
          }}
        >
          <div style={{ fontFamily: 'monospace', fontSize: 9, opacity: 0.6 }}>NEXT</div>
          <PiecePreview kind={nextPieceKind} cellSize={12} />
          <SelectionTimer ref={selectionTimerRef} width={48} />
        </div>
        <div style={{ gridColumn: 3, gridRow: 2 }}>
          <ArmPanel
            ref={boardRefs[WasmArm.East]}
            label="E"
            cellSize={cellSize}
            score={armHud[WasmArm.East].score}
            gameOver={armHud[WasmArm.East].gameOver}
            highlight={highlightFor(WasmArm.East)}
          />
        </div>
        <div style={{ gridColumn: 2, gridRow: 3 }}>
          <ArmPanel
            ref={boardRefs[WasmArm.South]}
            label="S"
            cellSize={cellSize}
            score={armHud[WasmArm.South].score}
            gameOver={armHud[WasmArm.South].gameOver}
            highlight={highlightFor(WasmArm.South)}
          />
        </div>
      </div>
      <div
        style={{
          display: 'flex',
          flexDirection: 'column',
          gap: 24,
          maxHeight: `calc(100dvh - ${OUTER_PADDING * 2}px)`,
          overflowY: 'auto',
        }}
      >
        <Hud totalScore={totalScore} gameOver={gameOver} nextPieceKind={nextPieceKind} awaitingSelection={awaitingSelection && !aiEnabled} />
        <Controls aiEnabled={aiEnabled} onToggleAi={() => setAiEnabled((v) => !v)} onRestart={startNewGame} />
      </div>
      <Credits />
    </div>
  )
}
