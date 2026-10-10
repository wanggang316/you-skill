/** Browser-style back and forward through the app's history (⌘[ and ⌘]). */

export function goBack(): void {
  history.back();
}

export function goForward(): void {
  history.forward();
}
