export interface LigatureAnimation {
  readonly duration: number;
  play(): void;
  finish(): void;
  seek(time: number): void;
  dispose(): void;
}
export function createLigatureAnimation(root: HTMLElement): LigatureAnimation;
