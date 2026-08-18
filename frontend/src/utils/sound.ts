// Audio sound manager using Lichess sound files
class SoundManager {
  private sounds: Map<string, HTMLAudioElement> = new Map();
  private enabled: boolean = true;

  constructor() {
    this.preloadSound('move', '/sound/standard/Move.mp3');
    this.preloadSound('capture', '/sound/standard/Capture.mp3');
    this.preloadSound('check', '/sound/standard/Check.mp3');
    this.preloadSound('checkmate', '/sound/standard/Checkmate.mp3');
    this.preloadSound('victory', '/sound/standard/Victory.mp3');
    this.preloadSound('defeat', '/sound/standard/Defeat.mp3');
    this.preloadSound('error', '/sound/standard/Error.mp3');
    this.preloadSound('confirmation', '/sound/standard/Confirmation.mp3');
  }

  private preloadSound(name: string, url: string) {
    try {
      const audio = new Audio(url);
      audio.preload = 'auto';
      this.sounds.set(name, audio);
    } catch {
      // Audio playback might be restricted before user interaction
    }
  }

  public play(name: string) {
    if (!this.enabled) return;
    try {
      const sound = this.sounds.get(name);
      if (sound) {
        sound.currentTime = 0;
        sound.play().catch(() => {
          // ignore autoplay policy errors
        });
      }
    } catch {
      // ignore
    }
  }

  public toggleMute(): boolean {
    this.enabled = !this.enabled;
    return this.enabled;
  }

  public isMuted(): boolean {
    return !this.enabled;
  }
}

export const sounds = new SoundManager();
