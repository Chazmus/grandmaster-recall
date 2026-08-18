import { describe, it, expect } from 'vitest';
import { sounds } from '../utils/sound';

describe('SoundManager', () => {
  it('should initialize and handle mute toggling', () => {
    expect(sounds.isMuted()).toBe(false);
    const muted = sounds.toggleMute();
    expect(muted).toBe(false); // returned current enabled state
    expect(sounds.isMuted()).toBe(true);

    sounds.toggleMute();
    expect(sounds.isMuted()).toBe(false);
  });

  it('should safely play sounds without crashing', () => {
    expect(() => {
      sounds.play('move');
      sounds.play('capture');
      sounds.play('victory');
      sounds.play('error');
    }).not.toThrow();
  });
});
