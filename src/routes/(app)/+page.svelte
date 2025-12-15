<script lang="ts">
    import { createQuery } from "@tanstack/svelte-query";
    import {
        fetchNews,
        fetchTrendingMods,
        type NewsItem,
        type TrendingMod,
    } from "$lib/queries";
    import * as Card from "$lib/components/ui/card";
    import * as Carousel from "$lib/components/ui/carousel";
    import { AspectRatio } from "$lib/components/ui/aspect-ratio";
    import { Skeleton } from "$lib/components/ui/skeleton";
    import { marked } from "marked";
    import Prose from "$lib/components/Prose.svelte";
    import {
        StarlightIcon,
        ClockIcon,
        DownloadIcon,
    } from "$lib/components/icons";
    import { PUBLIC_API_URL } from "$env/static/public";

    const newsQuery = createQuery(() => ({
        queryKey: ["news"],
        queryFn: () => fetchNews(),
        staleTime: 1000 * 60 * 5, // 5 minutes
    }));

    const trendingModsQuery = createQuery(() => ({
        queryKey: ["mods", "trending"],
        queryFn: () => fetchTrendingMods(),
        staleTime: 1000 * 60 * 5, // 5 minutes
    }));
</script>

<div class="p-8 overflow-y-auto h-full scrollbar-styled">
    <h1
        class="text-4xl font-bold mb-6 text-center flex items-center justify-center gap-3"
    >
        Welcome to Starlight!
        <StarlightIcon class="w-10 h-10" />
    </h1>

    <!-- News Section -->
    <div class="mb-8">
        <div class="flex items-center justify-between mb-4 px-10">
            <h2 class="text-xl font-semibold flex items-center gap-2">News</h2>
        </div>

        {#if newsQuery.isLoading}
            <Carousel.Root
                opts={{
                    align: "start",
                }}
                class="w-full maw-w-sm"
            >
                <Carousel.Content>
                    {#each Array(3) as _, i (i)}
                        <Carousel.Item class="md:basis-1/2 lg:basis-1/3">
                            <Card.Root class="flex flex-col h-96">
                                <Card.Content
                                    class="flex-1 overflow-hidden space-y-3 pt-6"
                                >
                                    <Skeleton class="h-6 w-3/4" />
                                    <Skeleton class="h-4 w-full" />
                                    <Skeleton class="h-4 w-full" />
                                    <Skeleton class="h-4 w-5/6" />
                                    <Skeleton class="h-4 w-full" />
                                    <Skeleton class="h-4 w-4/5" />
                                </Card.Content>
                                <Card.Footer
                                    class="text-xs flex items-end justify-between border-t border-border"
                                >
                                    <div class="flex flex-col gap-2 flex-1">
                                        <Skeleton class="h-4 w-32" />
                                        <Skeleton class="h-3 w-24" />
                                    </div>
                                    <Skeleton class="h-3 w-20" />
                                </Card.Footer>
                            </Card.Root>
                        </Carousel.Item>
                    {/each}
                </Carousel.Content>
            </Carousel.Root>
        {:else if newsQuery.isError}
            <div
                class="p-4 bg-error-background border border-error-border rounded-lg"
            >
                <p class="text-error font-semibold">Error loading news</p>
                <p class="text-error-foreground text-sm mt-1">
                    {newsQuery.error?.message}
                </p>
            </div>
        {:else if newsQuery.isSuccess && newsQuery.data}
            <Carousel.Root
                opts={{
                    align: "start",
                }}
                class="w-full maw-w-sm px-10"
            >
                <Carousel.Content class="-ml-2">
                    {#each newsQuery.data as newsItem: NewsItem (newsItem.id)}
                        <Carousel.Item class="md:basis-1/2 lg:basis-1/3">
                            <Card.Root class="flex flex-col h-96">
                                <Card.Content
                                    class="flex-1 overflow-hidden hover:overflow-y-auto scrollbar-styled"
                                >
                                    <Prose content={marked(newsItem.content)} />
                                </Card.Content>
                                <Card.Footer
                                    class="text-xs flex items-end justify-between border-t border-border"
                                >
                                    <div class="flex flex-col gap-0.5">
                                        <div
                                            class="font-semibold text-sm text-card-foreground"
                                        >
                                            {newsItem.title}
                                        </div>
                                        <div class="text-muted-foreground">
                                            {newsItem.author}
                                        </div>
                                    </div>
                                    <div class="text-muted-foreground">
                                        {new Date(
                                            newsItem.updated_at,
                                        ).toLocaleDateString()}
                                    </div>
                                </Card.Footer>
                            </Card.Root>
                        </Carousel.Item>
                    {/each}
                </Carousel.Content>
                <Carousel.Previous class="-left-2" />
                <Carousel.Next class="-right-2" />
            </Carousel.Root>
        {/if}
    </div>

    <!-- Trending Mods Section -->
    <div class="mb-8">
        <div class="flex items-center justify-between mb-4 px-10">
            <h2 class="text-xl font-semibold flex items-center gap-2">
                Trending Mods
            </h2>
        </div>

        {#if trendingModsQuery.isLoading}
            <Carousel.Root
                opts={{
                    align: "start",
                }}
                class="w-full maw-w-sm px-10"
            >
                <Carousel.Content class="-ml-2">
                    {#each Array(3) as _, i (i)}
                        <Carousel.Item class="md:basis-1/2 lg:basis-1/3">
                            <Card.Root
                                class="flex flex-col h-96 max-w-sm mx-auto"
                            >
                                <Card.Content
                                    class="flex-1 flex items-center justify-center p-0 overflow-hidden"
                                >
                                    <Skeleton
                                        class="w-full aspect-square max-h-full"
                                    />
                                </Card.Content>
                                <Card.Footer
                                    class="flex flex-col gap-2 border-t border-border"
                                >
                                    <div
                                        class="flex items-end justify-between w-full"
                                    >
                                        <div class="flex flex-col gap-1 flex-1">
                                            <Skeleton class="h-4 w-32" />
                                            <Skeleton class="h-3 w-24" />
                                        </div>
                                        <Skeleton class="h-3 w-16" />
                                    </div>
                                </Card.Footer>
                            </Card.Root>
                        </Carousel.Item>
                    {/each}
                </Carousel.Content>
            </Carousel.Root>
        {:else if trendingModsQuery.isError}
            <div
                class="p-4 bg-error-background border border-error-border rounded-lg"
            >
                <p class="text-error font-semibold">
                    Error loading trending mods
                </p>
                <p class="text-error-foreground text-sm mt-1">
                    {trendingModsQuery.error?.message}
                </p>
            </div>
        {:else if trendingModsQuery.isSuccess && trendingModsQuery.data}
            <Carousel.Root
                opts={{
                    align: "start",
                }}
                class="w-full maw-w-sm px-10"
            >
                <Carousel.Content class="-ml-2">
                    {#each trendingModsQuery.data as mod (mod.mod_id)}
                        <Carousel.Item class="md:basis-1/2 lg:basis-1/3">
                            <Card.Root class="overflow-hidden">
                                <div class="flex h-40">
                                    <!-- Thumbnail on the left - Square aspect ratio -->
                                    <div class="w-40 h-40 shrink-0">
                                        {#if mod.thumbnail}
                                            <AspectRatio
                                                ratio={1}
                                                class="bg-muted"
                                            >
                                                <img
                                                    src="{PUBLIC_API_URL}{mod.thumbnail}"
                                                    alt={mod.mod_name}
                                                    class="rounded-md object-cover"
                                                />
                                            </AspectRatio>
                                        {:else}
                                            <div
                                                class="h-full w-full bg-muted flex items-center justify-center"
                                            >
                                                <span
                                                    class="text-muted-foreground text-sm"
                                                    >No image</span
                                                >
                                            </div>
                                        {/if}
                                    </div>

                                    <!-- Data on the right -->
                                    <div
                                        class="flex-1 p-4 flex flex-col justify-between min-w-0"
                                    >
                                        <div class="space-y-1">
                                            <h3
                                                class="font-semibold text-lg leading-tight truncate"
                                                title={mod.mod_name}
                                            >
                                                {mod.mod_name}
                                            </h3>
                                            <p
                                                class="text-sm text-muted-foreground truncate"
                                                title={mod.author}
                                            >
                                                by {mod.author}
                                            </p>
                                        </div>

                                        <div
                                            class="space-y-2 text-sm text-muted-foreground"
                                        >
                                            <div
                                                class="flex items-center gap-2"
                                            >
                                                <DownloadIcon
                                                    class="h-4 w-4 flex-shrink-0"
                                                />
                                                <span
                                                    >{mod.downloads.toLocaleString()}
                                                    downloads</span
                                                >
                                            </div>
                                            <div
                                                class="flex items-center gap-2"
                                            >
                                                <ClockIcon
                                                    class="h-4 w-4 flex-shrink-0"
                                                />
                                                <span
                                                    >{new Date(
                                                        mod.created_at,
                                                    ).toLocaleDateString()}</span
                                                >
                                            </div>
                                        </div>
                                    </div>
                                </div>
                            </Card.Root>
                        </Carousel.Item>
                    {/each}
                </Carousel.Content>
                <Carousel.Previous class="-left-2" />
                <Carousel.Next class="-right-2" />
            </Carousel.Root>
        {/if}
    </div>
</div>
