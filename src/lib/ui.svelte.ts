export const uiState = $state({
  openNewMovementRequested: false,
});

export function requestNewMovement(): void {
  uiState.openNewMovementRequested = true;
}
