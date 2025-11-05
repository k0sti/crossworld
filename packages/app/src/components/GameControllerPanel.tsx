import { useEffect, useRef, useState } from 'react';
import { Box } from '@chakra-ui/react';

interface GameControllerPanelProps {
  width?: number;
  height?: number;
}

// Standard gamepad button names (per Gamepad API standard mapping)
const BUTTON_NAMES: Record<number, string> = {
  0: 'A', 1: 'B', 2: 'X', 3: 'Y',
  4: 'LB', 5: 'RB', 6: 'LT', 7: 'RT',
  8: 'Back', 9: 'Start', 10: 'LS', 11: 'RS',
  12: 'Up', 13: 'Down', 14: 'Left', 15: 'Right',
  16: 'Home'
};

const AXIS_NAMES: Record<number, string> = {
  0: 'LS-X', 1: 'LS-Y', 2: 'RS-X', 3: 'RS-Y'
};

export function GameControllerPanel({ width = 400, height = 300 }: GameControllerPanelProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const animationFrameRef = useRef<number>();
  const [gamepadConnected, setGamepadConnected] = useState(false);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    // Set canvas size
    canvas.width = width;
    canvas.height = height;

    const drawController = () => {
      // Clear canvas
      ctx.fillStyle = '#1a1a1a';
      ctx.fillRect(0, 0, width, height);

      // Get gamepad state - always poll fresh
      const gamepads = navigator.getGamepads();
      const gamepad = gamepads[0] || gamepads[1] || gamepads[2] || gamepads[3];

      if (!gamepad) {
        setGamepadConnected(false);
        // Draw "No controller connected" message
        ctx.fillStyle = '#666';
        ctx.font = '14px monospace';
        ctx.textAlign = 'center';
        ctx.fillText('No controller connected', width / 2, height / 2 - 10);

        // Check if any gamepads exist but are null (not activated)
        const gamepadCount = gamepads.filter(g => g !== null).length;
        const totalSlots = gamepads.length;

        if (gamepadCount === 0 && totalSlots > 0) {
          ctx.fillStyle = '#888';
          ctx.font = '12px monospace';
          ctx.fillText('Press any button on your controller', width / 2, height / 2 + 15);
          ctx.fillText('to activate it', width / 2, height / 2 + 30);
        }

        // Debug info
        ctx.font = '10px monospace';
        ctx.fillStyle = '#444';
        ctx.fillText(`Detected: ${gamepadCount}/${totalSlots} slots`, width / 2, height / 2 + 60);

        // Continue polling even when no gamepad
        animationFrameRef.current = requestAnimationFrame(drawController);
        return;
      }

      setGamepadConnected(true);

      // Draw title
      ctx.fillStyle = '#fff';
      ctx.font = 'bold 12px monospace';
      ctx.textAlign = 'left';
      ctx.fillText(`Controller: ${gamepad.id.substring(0, 40)}`, 10, 20);

      // Draw axes (left section)
      ctx.fillStyle = '#aaa';
      ctx.font = '11px monospace';
      ctx.fillText('AXES', 10, 45);

      const axesStartY = 60;

      gamepad.axes.forEach((value, index) => {
        const y = axesStartY + index * 25;
        const barWidth = 150;
        const barHeight = 16;

        // Axis label with name
        ctx.fillStyle = '#888';
        ctx.font = '9px monospace';
        const axisName = AXIS_NAMES[index] || `${index}`;
        ctx.fillText(axisName, 10, y + 12);

        // Background bar
        ctx.fillStyle = '#333';
        ctx.fillRect(30, y, barWidth, barHeight);

        // Value bar (centered at 0)
        const centerX = 30 + barWidth / 2;

        if (value < 0) {
          // Negative value - draw from center to left
          const barX = centerX + (value * barWidth / 2);
          const barW = Math.abs(value * barWidth / 2);
          ctx.fillStyle = '#4a9eff';
          ctx.fillRect(barX, y, barW, barHeight);
        } else {
          // Positive value - draw from center to right
          const barW = value * barWidth / 2;
          ctx.fillStyle = '#4a9eff';
          ctx.fillRect(centerX, y, barW, barHeight);
        }

        // Center line
        ctx.strokeStyle = '#666';
        ctx.lineWidth = 1;
        ctx.beginPath();
        ctx.moveTo(centerX, y);
        ctx.lineTo(centerX, y + barHeight);
        ctx.stroke();

        // Value text
        ctx.fillStyle = '#fff';
        ctx.font = '9px monospace';
        ctx.textAlign = 'right';
        ctx.fillText(value.toFixed(3), 185, y + 12);
      });

      // Draw buttons (right section)
      const buttonsX = 210;
      ctx.fillStyle = '#aaa';
      ctx.font = '11px monospace';
      ctx.textAlign = 'left';
      ctx.fillText('BUTTONS', buttonsX, 45);

      const buttonsStartY = 60;
      const buttonSize = 20;
      const buttonSpacing = 6;
      const buttonsPerRow = 4;

      gamepad.buttons.forEach((button, index) => {
        const col = index % buttonsPerRow;
        const row = Math.floor(index / buttonsPerRow);
        const x = buttonsX + col * (buttonSize + buttonSpacing);
        const y = buttonsStartY + row * (buttonSize + buttonSpacing);

        // Button circle
        ctx.beginPath();
        ctx.arc(x + buttonSize / 2, y + buttonSize / 2, buttonSize / 2, 0, Math.PI * 2);

        // Color based on press state
        if (button.pressed) {
          ctx.fillStyle = '#ff4444';
        } else if (button.touched) {
          ctx.fillStyle = '#ffaa44';
        } else {
          ctx.fillStyle = '#333';
        }
        ctx.fill();

        // Button outline
        ctx.strokeStyle = button.value > 0 ? '#fff' : '#666';
        ctx.lineWidth = 1;
        ctx.stroke();

        // Button number
        ctx.fillStyle = '#fff';
        ctx.font = 'bold 9px monospace';
        ctx.textAlign = 'center';
        ctx.fillText(index.toString(), x + buttonSize / 2, y + buttonSize / 2 + 3);

        // Value indicator (for analog buttons)
        if (button.value > 0 && button.value < 1) {
          ctx.fillStyle = 'rgba(255, 255, 255, 0.3)';
          ctx.beginPath();
          ctx.arc(x + buttonSize / 2, y + buttonSize / 2, (buttonSize / 2) * button.value, 0, Math.PI * 2);
          ctx.fill();
        }

        // Button name (very small below button)
        const buttonName = BUTTON_NAMES[index];
        if (buttonName) {
          ctx.fillStyle = '#666';
          ctx.font = '7px monospace';
          ctx.textAlign = 'center';
          ctx.fillText(buttonName, x + buttonSize / 2, y + buttonSize + 8);
        }
      });

      // Request next frame
      animationFrameRef.current = requestAnimationFrame(drawController);
    };

    // Start animation loop
    drawController();

    // Gamepad connection events
    const handleGamepadConnected = (e: GamepadEvent) => {
      console.log('Gamepad connected:', e.gamepad.id);
    };

    const handleGamepadDisconnected = (e: GamepadEvent) => {
      console.log('Gamepad disconnected:', e.gamepad.id);
    };

    window.addEventListener('gamepadconnected', handleGamepadConnected);
    window.addEventListener('gamepaddisconnected', handleGamepadDisconnected);

    return () => {
      if (animationFrameRef.current) {
        cancelAnimationFrame(animationFrameRef.current);
      }
      window.removeEventListener('gamepadconnected', handleGamepadConnected);
      window.removeEventListener('gamepaddisconnected', handleGamepadDisconnected);
    };
  }, [width, height]);

  return (
    <Box
      position="relative"
      bg="gray.900"
      borderRadius="md"
      overflow="hidden"
      border="1px solid"
      borderColor={gamepadConnected ? 'green.500' : 'gray.700'}
    >
      <canvas
        ref={canvasRef}
        style={{
          display: 'block',
          width: '100%',
          height: 'auto',
        }}
      />
    </Box>
  );
}
