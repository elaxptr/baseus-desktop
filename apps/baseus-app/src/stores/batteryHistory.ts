import { createStore } from 'solid-js/store';

export interface Reading {
  pct: number;
  ts: number;
}

interface HistoryState {
  left: Reading[];
  right: Reading[];
  case: Reading[];
}

const MAX = 60;

const [history, setHistory] = createStore<HistoryState>({
  left: [],
  right: [],
  case: [],
});

function push(key: keyof HistoryState, pct: number) {
  setHistory(key, (prev) => {
    const next = [...prev, { pct, ts: Date.now() }];
    return next.length > MAX ? next.slice(next.length - MAX) : next;
  });
}

export function pushLeft(pct: number) {
  push('left', pct);
}

export function pushRight(pct: number) {
  push('right', pct);
}

export function pushCase(pct: number) {
  push('case', pct);
}

export function getLeft(): Reading[] {
  return history.left;
}

export function getRight(): Reading[] {
  return history.right;
}

export function getCase(): Reading[] {
  return history.case;
}
