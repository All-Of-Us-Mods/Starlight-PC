let activeMutationsCount = 0;

export function getActiveMutationsCount() {
  return activeMutationsCount;
}

export function isProfileMutationInFlight() {
  return activeMutationsCount > 0;
}

export function withProfileMutationTracking<T>(fn: () => Promise<T>): Promise<T> {
  activeMutationsCount++;
  return fn().finally(() => {
    activeMutationsCount--;
  });
}
