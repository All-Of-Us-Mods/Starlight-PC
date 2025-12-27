# Implementation Plan: Mod Installation to Profiles

## Overview

This plan covers adding the ability to install mods to profiles, manage mod versions, and display which mods are installed on each profile.

## Current State Analysis

**Existing Infrastructure:**

- ✅ Profile schema with `mods: ProfileModEntry[]` array
- ✅ ProfileService with `addModToProfile()` and `removeModFromProfile()` methods
- ✅ ModInstallService with download and installation logic
- ✅ AddToProfileDialog component (functional)
- ✅ Mod schemas (Mod, ModVersion, ModVersionInfo, ModDependency)
- ✅ ModCard component (display only)
- ✅ API queries for mod versions and info

**Missing Features:**

- ❌ "Add to Profile" button on ModCard
- ❌ Mod detail page (`/mods/[id]`)
- ❌ Display installed mods on ProfileCard
- ❌ Remove mod from profile functionality
- ❌ Dependency validation before install
- ❌ Version history display
- ❌ Installation progress tracking

---

## Phase 1: Update ProfileCard to Show Installed Mods

### 1.1 Add Mods List to ProfileCard

**File:** `src/lib/features/profiles/components/ProfileCard.svelte`

**Changes:**

- Display list of installed mods (first 3 with "X more" overflow)
- Add pill/badge for each mod
- Show "0 mods" state
- Collapse/expand button to see all mods
- Add "Manage Mods" dropdown menu

### 1.2 ProfileCard Actions

**File:** `src/lib/features/profiles/components/ProfileCard.svelte`

**Features:**

- "Manage Mods" dropdown with:
  - List of all installed mods
  - "Remove" button for each mod
  - Confirmation dialog before removal

### 1.3 Update Profile Schema

**File:** `src/lib/features/profiles/schema.ts`

**Enhancement:**
Add computed fields for mod metadata:

```ts
export type ProfileModEntry = {
	mod_id: string;
	version: string;
	// Future: installed_at timestamp
};
```

---

## Phase 2: Add "Add to Profile" to ModCard

### 2.1 Update ModCard Component

**File:** `src/lib/features/mods/components/ModCard.svelte`

**Changes:**

- Add action bar with "Add to Profile" button
- Import and use AddToProfileDialog component
- Style button to match existing design
- Disabled state when no profiles exist

### 2.2 Add Profile Count Query

**File:** `src/lib/features/profiles/queries.ts`

**New Query:**

```ts
hasProfiles: () => ({
	queryKey: ['profiles', 'has-any'],
	queryFn: async () => {
		const profiles = await profileService.getProfiles();
		return profiles.length > 0;
	}
});
```

### 2.3 Update Explore Page

**File:** `src/routes/explore/+page.svelte`

**Changes:**

- Pass `modId` to ModCard
- Display "No profiles" prompt if user tries to add without profiles
- Redirect to `/library` if needed

---

## Phase 3: Create Mod Detail Page

### 3.1 Route Structure

**File:** `src/routes/mods/[id]/+page.svelte`

**Layout:**

```
┌─────────────────────────────────────┐
│ Mod Header                          │
│  - Thumbnail                        │
│  - Name, Author                     │
│  - Downloads, Updated Date          │
│  - "Add to Profile" button          │
├─────────────────────────────────────┤
│ Description                         │
│  - Long description from API       │
│  - Links (GitHub, Website, etc.)    │
├─────────────────────────────────────┤
│ Versions                            │
│  - List of all versions             │
│  - Changelog for each              │
│  - Dependencies for each            │
├─────────────────────────────────────┤
│ Installation Status                 │
│  - Which profiles have this mod     │
│  - What version on each profile     │
└─────────────────────────────────────┘
```

### 3.2 Create ModDetail Component

**File:** `src/lib/features/mods/components/ModDetail.svelte`

**Sections:**

- **Header:** Mod name, author, stats
- **Description:** Long description, external links
- **Versions:** Accordion/collapsible version list
- **Dependencies:** Show tree or list of dependencies
- **Installed On:** List of profiles with this mod installed

### 3.3 Add Version History Component

**File:** `src/lib/features/mods/components/VersionHistory.svelte`

**Features:**

- List all versions sorted by date (newest first)
- Show changelog for each version
- Highlight currently selected version
- Show dependencies with version constraints
- "View" and "Install" buttons

### 3.4 Add Dependency Tree Component

**File:** `src/lib/features/mods/components/DependencyTree.svelte`

**Features:**

- Visual tree of mod dependencies
- Show installed vs missing dependencies
- Version constraint satisfaction check
- Click to install missing dependencies

### 3.5 Add API Queries

**File:** `src/lib/features/mods/queries.ts`

**New Queries:**

```ts
versions: (modId: string) => ({
  queryKey: ['mods', 'versions', modId],
  queryFn: () => apiFetch(`/api/v2/mods/${modId}/versions`, ModVersionsArray)
}),

installedProfiles: (modId: string) => ({
  queryKey: ['mods', 'installed-profiles', modId],
  queryFn: async () => {
    const profiles = await profileService.getProfiles();
    return profiles.filter(p => p.mods.some(m => m.mod_id === modId));
  }
})
```

---

## Phase 4: Enhanced AddToProfileDialog

### 4.1 Show Dependencies

**File:** `src/lib/features/profiles/components/AddToProfileDialog.svelte`

**Changes:**

- Fetch version info before showing install
- Display dependencies list with version constraints
- Check if dependencies are satisfied:
  - Already installed on profile?
  - Compatible version installed?
  - Missing entirely?
- Warning if dependencies unsatisfied
- "Install Dependencies" checkbox

### 4.2 Dependency Resolution

**File:** `src/lib/features/profiles/mod-install-service.ts`

**New Methods:**

```ts
async checkDependencies(
  modId: string,
  version: string,
  profile: Profile
): Promise<{
  satisfied: boolean;
  missing: ModDependency[];
  conflicting: { dep: ModDependency; installed: ProfileModEntry }[];
}> {
  // Fetch dependencies for version
  // Check against profile's installed mods
  // Return satisfaction status
}

async installWithDependencies(
  modId: string,
  version: string,
  profilePath: string,
  options: { includeDeps?: boolean }
): Promise<void> {
  // Install main mod
  // Optionally install missing dependencies
}
```

### 4.3 Improved UX

**Features:**

- Show dependency tree in dialog
- Green checkmark for satisfied deps
- Red warning for missing/deps
- "Install all required" button
- Progress indicator for multi-mod install

---

## Phase 5: Remove Mod from Profile

### 5.1 Add Remove Button

**File:** `src/lib/features/profiles/components/ProfileCard.svelte`

**Location:**

- Inside "Manage Mods" dropdown menu
- Next to each installed mod in the list

### 5.2 Confirmation Dialog

**File:** `src/lib/features/profiles/components/RemoveModDialog.svelte`

**New Component:**

```svelte
<Dialog.Root>
	<Dialog.Content>
		<Dialog.Header>
			<Dialog.Title>Remove Mod from Profile?</Dialog.Title>
			<Dialog.Description>
				This will remove {modName} from {profileName}.
			</Dialog.Description>
		</Dialog.Header>

		{#if hasDependents}
			<Alert>
				<Warning />
				<AlertTitle>Other mods depend on this mod</AlertTitle>
				<AlertDescription>
					The following mods may stop working: {dependentNames}
				</AlertDescription>
			</Alert>
		{/if}

		<div class="flex justify-end gap-2">
			<Dialog.Close><Button variant="outline">Cancel</Button></Dialog.Close>
			<Button onclick={handleRemove}>Remove</Button>
		</div>
	</Dialog.Content>
</Dialog.Root>
```

### 5.3 Dependency Check Before Removal

**File:** `src/lib/features/profiles/mod-install-service.ts`

**New Method:**

```ts
async checkDependents(
  modId: string,
  profileId: string
): Promise<string[]> {
  // Get profile's installed mods
  // Check dependencies of each mod
  // Return list of mods that depend on modId
}
```

### 5.4 Update ProfileService

**File:** `src/lib/features/profiles/profile-service.ts`

**Change:** Already has `removeModFromProfile()` - add dependency validation

---

## Phase 6: Query Integration

### 6.1 Update modQueries

**File:** `src/lib/features/mods/queries.ts`

**Add:**

```ts
versions: (modId: string) => queryOptions({
  queryKey: ['mods', 'versions', modId],
  queryFn: () => apiFetch(`/api/v2/mods/${modId}/versions`, ModVersionsArray),
  staleTime: 1000 * 60 * 5
}),

versionInfo: (modId: string, version: string) => queryOptions({
  queryKey: ['mods', 'version-info', modId, version],
  queryFn: () => apiFetch(`/api/v2/mods/${modId}/versions/${version}/info`, ModVersionInfo),
  staleTime: 1000 * 60 * 30
})
```

### 6.2 Add Optimistic Updates

**Pattern:** Use in AddToProfileDialog and ProfileCard removal

```ts
// In component
const queryClient = useQueryClient();

async function handleInstall() {
	const previous = queryClient.getQueryData<Profile[]>(['profiles']);

	// Optimistic update
	queryClient.setQueryData(['profiles'], (old = []) =>
		(old as Profile[]).map((p) =>
			p.id === profileId
				? {
						...p,
						mods: [...p.mods, { mod_id, version }]
					}
				: p
		)
	);

	try {
		await modInstallService.installModToProfile(modId, version, profile.path);
	} catch (e) {
		queryClient.setQueryData(['profiles'], previous);
		throw e;
	}
}
```

---

## Phase 7: Data Flow Diagrams

### 7.1 Install Mod Flow

```
User clicks "Add to Profile"
  ↓
AddToProfileDialog opens
  ↓
User selects profile & version
  ↓
Fetch version info (dependencies)
  ↓
Display dependency check
  ↓
User confirms install
  ↓
[Optimistic] Update query cache
  ↓
Download mod DLL (verify checksum)
  ↓
Write to profile/BepInEx/plugins/
  ↓
Update profile store (mods array)
  ↓
Invalidate profiles query
```

### 7.2 Remove Mod Flow

```
User clicks "Remove" in ProfileCard
  ↓
RemoveModDialog opens
  ↓
Check for dependent mods
  ↓
Show warning if dependents exist
  ↓
User confirms removal
  ↓
[Optimistic] Update query cache
  ↓
Delete mod DLL from profile
  ↓
Update profile store (remove from mods array)
  ↓
Invalidate profiles query
```

---

## Phase 8: Error Handling & Edge Cases

### 8.1 Error Scenarios

- **Network error during download:** Show retry button
- **Checksum mismatch:** Show error, don't install
- **Disk full:** Show error message
- **Permission denied:** Show error, check write access
- **Profile deleted during install:** Handle gracefully
- **Dependency version conflict:** Show conflict, let user resolve

### 8.2 Loading States

- Show spinner during download
- Show progress bar for large files
- Disable buttons during installation
- Show "Installing..." text

### 8.3 Empty States

- No profiles: Prompt to create profile first
- No mods on profile: Show "No mods installed"
- No versions available: Show "No versions available"

---

## Phase 9: Testing Checklist

### 9.1 Install Mod

- [ ] Dialog opens correctly
- [ ] Profile dropdown shows all profiles
- [ ] Version dropdown shows all versions
- [ ] Dependencies display correctly
- [ ] Install works with valid data
- [ ] Error handling on network failure
- [ ] Optimistic update works
- [ ] Rollback on error

### 9.2 Remove Mod

- [ ] Remove dialog opens
- [ ] Dependent mods detected
- [ ] Warning shown for dependents
- [ ] Removal works
- [ ] ProfileCard updates
- [ ] File deleted from disk

### 9.3 UI Components

- [ ] ProfileCard shows mods correctly
- [ ] ModCard has "Add to Profile" button
- [ ] Mod detail page loads
- [ ] Version history displays
- [ ] Dependency tree renders

### 9.4 Edge Cases

- [ ] No profiles → create profile prompt
- [ ] No versions → handle gracefully
- [ ] Circular dependencies → detect and warn
- [ ] Version constraint mismatch → show conflict
- [ ] Concurrent installs → handle race conditions

---

## Phase 10: Implementation Priority

**MVP (Minimum Viable Product):**

1. Add "Add to Profile" button to ModCard
2. Update ProfileCard to show installed mods
3. Add remove mod functionality
4. Basic dependency display in AddToProfileDialog

**Phase 2 (Enhanced):** 5. Create mod detail page 6. Version history display 7. Dependency tree visualization 8. Dependent mod checking on removal

**Phase 3 (Polish):** 9. Installation progress tracking 10. Batch install dependencies 11. Conflict resolution UI 12. Better error messages and recovery

---

## File Structure

```
src/lib/features/
├── mods/
│   ├── components/
│   │   ├── ModCard.svelte          # UPDATE: Add "Add to Profile" button
│   │   ├── ModDetail.svelte        # NEW: Mod detail page component
│   │   ├── VersionHistory.svelte  # NEW: Version list with changelog
│   │   └── DependencyTree.svelte   # NEW: Visual dependency tree
│   └── queries.ts                  # UPDATE: Add version info queries
└── profiles/
    ├── components/
    │   ├── ProfileCard.svelte     # UPDATE: Show installed mods, remove button
    │   ├── AddToProfileDialog.svelte # UPDATE: Show dependencies
    │   └── RemoveModDialog.svelte  # NEW: Confirmation dialog
    ├── profile-service.ts          # UPDATE: Enhanced remove with deps check
    └── mod-install-service.ts     # UPDATE: Dependency resolution

src/routes/
└── mods/
    └── [id]/
        └── +page.svelte            # NEW: Mod detail page
```

---

## Summary

This plan provides a comprehensive approach to adding mod installation functionality to the profiles system. The MVP can be completed with minimal changes to existing infrastructure, while the enhanced phases add valuable user experience improvements like dependency visualization and version management.

Key design decisions:

1. **Optimistic updates** for instant UI feedback
2. **Dependency validation** to prevent broken installs
3. **Rollback on error** to maintain data consistency
4. **Progressive enhancement** - MVP first, polish later
