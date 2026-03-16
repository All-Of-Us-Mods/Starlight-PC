import { type } from 'arktype';

export const Post = type({
	id: 'number',
	title: 'string',
	author: 'string',
	content: 'string',
	'tags?': 'string[]',
	created_at: 'number',
	updated_at: 'number'
});

export type Post = typeof Post.infer;
