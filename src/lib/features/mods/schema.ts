import { type } from "arktype";

const ExternalLink = type({
  type: "string",
  url: "string",
});

const ModResponseLinks = type({
  self: "string",
  thumbnail: "string",
  versions: "string",
});

export const ModResponse = type({
  "status?": "string", // Maps to db.PublicationStatus
  "mod_type?": "string",
  id: "string <= 100",
  name: "string <= 100",
  author: "string <= 100",
  description: "string <= 500",
  "long_description?": "string <= 20000",
  "license?": "string <= 100",
  "links?": type(ExternalLink.array()),
  "tags?": "string[]",
  created_at: "number",
  updated_at: "number",
  downloads: "number",
  _links: ModResponseLinks,
});

const ModInfoResponse = ModResponse;

const ModDependency = type({
  mod_id: "string",
  name: "string",
  version_constraint: "string",
  type: "'required' | 'optional' | 'conflict'",
});

export const ModVersionInfo = type({
  "status?": "string",
  name: "string",
  version: "string",
  "supported_platforms?": "string[]",
  downloads: "number",
  created_at: "number",
  "changelog?": "string",
  "platforms?": type(
    type({
      platform: "string",
      architecture: "string",
      "file_name?": "string",
      "file_size?": "number",
      "checksum?": "string",
      "download_url?": "string",
    }).array(),
  ),
  dependencies: type(ModDependency.array()),
});

export const ModVersion = type({
  "status?": "string",
  name: "string",
  version: "string",
  "supported_platforms?": "string[]",
  downloads: "number",
  created_at: "number",
  _links: {
    self: "string",
  },
});

// TypeScript Types
export type Mod = typeof ModResponse.infer;
export type ModInfo = typeof ModInfoResponse.infer;
export type ExternalLink = typeof ExternalLink.infer;
export type ModDependency = typeof ModDependency.infer;
export type ModVersionInfo = typeof ModVersionInfo.infer;
export type ModVersion = typeof ModVersion.infer;
