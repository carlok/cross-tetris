import { useCallback, useEffect, useRef, useState } from 'react'
import './App.css'
import { Board, type BoardHandle } from './components/Board'
import { Hud } from './components/Hud'
import { Controls } from './components/Controls'
import { useGameLoop } from './game/useGameLoop'
import { useKeyboardInput } from './game/useKeyboardInput'
import { initWasm, WasmGame } from './wasm'

const NEXT_QUEUE_SIZE = 5

function randomSeed(): bigint {
  return BigInt(Math.floor(Math.random() * Number.MAX_SAFE_INTEGER))
}

export default function App() {
  const [ready, setReady] = useState(false)
  const [aiEnabled, setAiEnabled] = useState(false)
  const [hud, setHud] = useState({ score: 0, level: 1, linesCleared: 0, gameOver: false, holdKind: -1, nextQueue: [] as number[] })

  const gameRef = useRef<WasmGame | null>(null)
  const boardRef = useRef<BoardHandle>(null)
  const hudRef = useRef(hud)

  const startNewGame = useCallback(() => {
    gameRef.current?.free()
    gameRef.current = new WasmGame(randomSeed())
    setHud({ score: 0, level: 1, linesCleared: 0, gameOver: false, holdKind: -1, nextQueue: [] })
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

    boardRef.current?.draw(game.board_buffer())

    const next = {
      score: game.score(),
      level: game.level(),
      linesCleared: game.lines_cleared(),
      gameOver: game.is_game_over(),
      holdKind: game.hold_piece_kind(),
      nextQueue: Array.from(game.next_queue(NEXT_QUEUE_SIZE)),
    }
    const prev = hudRef.current
    if (
      next.score !== prev.score ||
      next.level !== prev.level ||
      next.linesCleared !== prev.linesCleared ||
      next.gameOver !== prev.gameOver ||
      next.holdKind !== prev.holdKind ||
      next.nextQueue.join(',') !== prev.nextQueue.join(',')
    ) {
      hudRef.current = next
      setHud(next)
    }
  }, [])

  useGameLoop(gameRef, aiEnabled, onFrame)
  useKeyboardInput(gameRef, ready && !aiEnabled)

  return (
    <div style={{ display: 'flex', gap: 24, padding: 24, alignItems: 'flex-start' }}>
      <Board ref={boardRef} />
      <div style={{ display: 'flex', flexDirection: 'column', gap: 24 }}>
        <Hud {...hud} />
        <Controls aiEnabled={aiEnabled} onToggleAi={() => setAiEnabled((v) => !v)} onRestart={startNewGame} />
      </div>
    </div>
  )
}
