export interface ResolvedDependency {
	mod_id: string;
	modName: string;
	resolvedVersion: string;
	type: 'required' | 'optional' | 'conflict';
}
