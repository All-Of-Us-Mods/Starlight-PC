import { queryOptions } from "@tanstack/svelte-query";
import { type } from "arktype";
import { apiFetch } from "$lib/infra/http/starlight-api";
import { Post } from "./schema";
import { newsByIdKey, newsQueryKey } from "./news-keys";

const PostsArray = type(Post.array());

export const newsQueries = {
  all: () =>
    queryOptions({
      queryKey: newsQueryKey,
      queryFn: () => apiFetch("/api/v3/news/posts", PostsArray),
      networkMode: "offlineFirst",
    }),
  byId: (id: string | number) =>
    queryOptions({
      queryKey: newsByIdKey(id),
      queryFn: () => apiFetch(`/api/v3/news/posts/${id}`, Post),
      networkMode: "offlineFirst",
    }),
};
