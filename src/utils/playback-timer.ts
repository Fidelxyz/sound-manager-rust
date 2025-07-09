export class PlaybackTimer {
  private startPos: number;
  private startTime: number;

  constructor() {
    this.startPos = 0;
    this.startTime = 0;
  }

  start(pos: number) {
    this.startPos = pos;
    this.startTime = Date.now();
  }

  pause() {
    this.startPos = this.getCurrentPos();
    this.startTime = 0;
  }

  getCurrentPos() {
    if (this.startTime === 0) {
      return this.startPos;
    }
    return this.startPos + (Date.now() - this.startTime) / 1000;
  }

  setPos(pos: number) {
    this.startPos = pos;
    if (this.isPlaying()) {
      this.startTime = Date.now();
    }
  }

  isPlaying() {
    return this.startTime !== 0;
  }
}
