import { useCallback, useEffect, useRef, useState } from 'react'
import './App.css'
import { ArmPanel, type ArmHighlight } from './components/ArmPanel'
import type { BoardHandle } from './components/Board'
import { Hud } from './components/Hud'
import { Controls } from './components/Controls'
import { useGameLoop } from './game/useGameLoop'
import { useKeyboardInput } from './game/useKeyboardInput'
import { initWasm, WasmArm, WasmCrossGame } from './wasm'

const CELL_SIZE = 16

const ARMS = [
  { arm: WasmArm.North, label: 'N' },
  { arm: WasmArm.East, label: 'E' },
  { arm: WasmArm.South, label: 'S' },
  { arm: WasmArm.West, label: 'W' },
] as const

type ArmHud = { score: number; gameOver: boolean }
const EMPTY_ARM_HUD: ArmHud = { score: 0, gameOver: false }
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

  const gameRef = useRef<WasmCrossGame | null>(null)
  const boardRefs = {
    [WasmArm.North]: useRef<BoardHandle>(null),
    [WasmArm.East]: useRef<BoardHandle>(null),
    [WasmArm.South]: useRef<BoardHandle>(null),
    [WasmArm.West]: useRef<BoardHandle>(null),
  }
  const hudRef = useRef({ totalScore, gameOver, awaitingSelection, activeArm, nextPieceKind, armHud })

  const startNewGame = useCallback(() => {
    gameRef.current?.free()
    gameRef.current = new WasmCrossGame(randomSeed())
    setTotalScore(0)
    setGameOver(false)
    setAwaitingSelection(true)
    setActiveArm(null)
    setNextPieceKind(0)
    setArmHud(EMPTY_ARM_HUD_MAP)
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

    const nextArmHud = Object.fromEntries(
      ARMS.map(({ arm }) => [arm, { score: game.score(arm), gameOver: game.arm_game_over(arm) }]),
    ) as Record<WasmArm, ArmHud>
    const rawActiveArm = game.active_arm()
    const next = {
      totalScore: game.total_score(),
      gameOver: game.is_game_over(),
      awaitingSelection: game.awaiting_well_selection(),
      activeArm: rawActiveArm >= 0 ? (rawActiveArm as WasmArm) : null,
      nextPieceKind: game.next_queue(1)[0] ?? 0,
      armHud: nextArmHud,
    }

    const prev = hudRef.current
    const armHudChanged = ARMS.some(
      ({ arm }) => prev.armHud[arm].score !== next.armHud[arm].score || prev.armHud[arm].gameOver !== next.armHud[arm].gameOver,
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
  }, [])

  useGameLoop(gameRef, aiEnabled, onFrame)
  useKeyboardInput(gameRef, ready && !aiEnabled)

  const highlightFor = (arm: WasmArm): ArmHighlight => {
    if (activeArm === arm) return 'active'
    if (awaitingSelection && !armHud[arm].gameOver && !aiEnabled) return 'selectable'
    return 'none'
  }

  return (
    <div style={{ display: 'flex', gap: 24, padding: 24, alignItems: 'flex-start' }}>
      <div
        style={{
          display: 'grid',
          gridTemplateColumns: 'repeat(3, auto)',
          gridTemplateRows: 'repeat(3, auto)',
          gap: 8,
          justifyItems: 'center',
          alignItems: 'center',
        }}
      >
        <div style={{ gridColumn: 2, gridRow: 1 }}>
          <ArmPanel
            ref={boardRefs[WasmArm.North]}
            label="N"
            cellSize={CELL_SIZE}
            score={armHud[WasmArm.North].score}
            gameOver={armHud[WasmArm.North].gameOver}
            highlight={highlightFor(WasmArm.North)}
          />
        </div>
        <div style={{ gridColumn: 1, gridRow: 2 }}>
          <ArmPanel
            ref={boardRefs[WasmArm.West]}
            label="W"
            cellSize={CELL_SIZE}
            score={armHud[WasmArm.West].score}
            gameOver={armHud[WasmArm.West].gameOver}
            highlight={highlightFor(WasmArm.West)}
          />
        </div>
        <div style={{ gridColumn: 2, gridRow: 2, fontFamily: 'monospace', fontSize: 12, opacity: 0.5 }}>+</div>
        <div style={{ gridColumn: 3, gridRow: 2 }}>
          <ArmPanel
            ref={boardRefs[WasmArm.East]}
            label="E"
            cellSize={CELL_SIZE}
            score={armHud[WasmArm.East].score}
            gameOver={armHud[WasmArm.East].gameOver}
            highlight={highlightFor(WasmArm.East)}
          />
        </div>
        <div style={{ gridColumn: 2, gridRow: 3 }}>
          <ArmPanel
            ref={boardRefs[WasmArm.South]}
            label="S"
            cellSize={CELL_SIZE}
            score={armHud[WasmArm.South].score}
            gameOver={armHud[WasmArm.South].gameOver}
            highlight={highlightFor(WasmArm.South)}
          />
        </div>
      </div>
      <div style={{ display: 'flex', flexDirection: 'column', gap: 24 }}>
        <Hud totalScore={totalScore} gameOver={gameOver} nextPieceKind={nextPieceKind} awaitingSelection={awaitingSelection && !aiEnabled} />
        <Controls aiEnabled={aiEnabled} onToggleAi={() => setAiEnabled((v) => !v)} onRestart={startNewGame} />
      </div>
    </div>
  )
}
