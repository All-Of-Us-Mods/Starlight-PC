class StartupState {
  #amongUsPathDialogOpen = $state(false);
  #detectedAmongUsPath = $state("");

  get amongUsPathDialogOpen() {
    return this.#amongUsPathDialogOpen;
  }

  get detectedAmongUsPath() {
    return this.#detectedAmongUsPath;
  }

  showAmongUsPathDialog(detectedPath = "") {
    this.#detectedAmongUsPath = detectedPath;
    this.#amongUsPathDialogOpen = true;
  }

  hideAmongUsPathDialog() {
    this.#amongUsPathDialogOpen = false;
  }
}

export const startupState = new StartupState();
