<script lang="ts">
  import { createQuery } from "@tanstack/svelte-query";
  import { newsQueries, modQueries } from "$lib/queries";
  import * as Card from "$lib/components/ui/card";
  import * as Carousel from "$lib/components/ui/carousel";
  import { Skeleton } from "$lib/components/ui/skeleton";
  import { marked } from "marked";
  import Prose from "$lib/components/Prose.svelte";
  import {
    Clock,
    Download,
    User,
    Calendar,
    ArrowRight,
    ImageOff,
  } from "@lucide/svelte";

  const newsQuery = createQuery(newsQueries.all);
  const trendingModsQuery = createQuery(modQueries.trending);
</script>

<div class="p-8 overflow-y-auto h-full scrollbar-styled space-y-12">
  <!-- News Section -->
  <section>
    <div class="mb-6 px-10">
      <h2 class="text-2xl font-bold tracking-tight">Latest News</h2>
      <p class="text-sm text-muted-foreground">
        Updates from the development team.
      </p>
    </div>

    {#if newsQuery.isLoading}
      <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6 px-10">
        {#each Array(3) as _}
          <Skeleton class="h-72 w-full rounded-xl" />
        {/each}
      </div>
    {:else if newsQuery.isSuccess}
      <Carousel.Root opts={{ align: "start" }} class="w-full px-10">
        <Carousel.Content class="-ml-4">
          {#each newsQuery.data as newsItem (newsItem.id)}
            <Carousel.Item class="pl-4 md:basis-1/2 lg:basis-1/3">
              <Card.Root
                class="group flex flex-col h-72 transition-all hover:border-primary/50 hover:shadow-lg"
              >
                <Card.Header class="pb-3">
                  <div
                    class="flex items-center justify-between mb-2 text-xs text-muted-foreground"
                  >
                    <div class="flex items-center gap-1.5">
                      <User class="w-3.5 h-3.5" />
                      <span class="font-medium text-foreground/80"
                        >{newsItem.author}</span
                      >
                    </div>
                    <div class="flex items-center gap-1.5">
                      <Calendar class="w-3.5 h-3.5" />
                      {new Date(newsItem.updated_at).toLocaleDateString()}
                    </div>
                  </div>

                  <Card.Title
                    class="text-lg leading-snug group-hover:text-primary transition-colors"
                  >
                    {newsItem.title}
                  </Card.Title>
                </Card.Header>

                <Card.Content class="flex-1 overflow-hidden relative px-6 py-0">
                  <div class="prose-sm text-muted-foreground">
                    <Prose content={marked(newsItem.content)} />
                  </div>

                  <div
                    class="absolute bottom-0 left-0 right-0 h-12 bg-linear-to-t from-card to-transparent pointer-events-none"
                  ></div>
                </Card.Content>

                <Card.Footer class="pt-2 pb-4">
                  <button
                    class="text-sm font-semibold text-primary hover:underline flex items-center gap-1 group/btn"
                  >
                    Read full story
                    <ArrowRight
                      class="w-3.5 h-3.5 transition-transform group-hover/btn:translate-x-1"
                    />
                  </button>
                </Card.Footer>
              </Card.Root>
            </Carousel.Item>
          {/each}
        </Carousel.Content>
        <Carousel.Previous class="-left-2" />
        <Carousel.Next class="-right-2" />
      </Carousel.Root>
    {/if}
  </section>

  <!-- Trending Mods Section -->
  <section>
    <div class="flex items-center justify-between mb-4 px-10">
      <div>
        <h2 class="text-2xl font-bold tracking-tight">Trending Mods</h2>
        <p class="text-muted-foreground text-sm">
          Most popular community creations this week.
        </p>
      </div>
    </div>

    {#if trendingModsQuery.isLoading}
      <Carousel.Root opts={{ align: "start" }} class="w-full px-10">
        <Carousel.Content class="-ml-2">
          {#each Array(3) as _}
            <Carousel.Item class="basis-full lg:basis-1/2 xl:basis-1/3">
              <Card.Root class="p-0 overflow-hidden">
                <div class="flex h-40">
                  <Skeleton class="w-40 h-40 shrink-0 rounded-none" />
                  <div class="flex-1 p-3 space-y-3">
                    <div class="space-y-1">
                      <Skeleton class="h-5 w-3/4" />
                      <Skeleton class="h-4 w-1/2" />
                    </div>
                    <div class="space-y-1">
                      <Skeleton class="h-4 w-full" />
                      <Skeleton class="h-4 w-full" />
                      <Skeleton class="h-4 w-2/3" />
                    </div>
                    <Skeleton class="h-4 w-1/3 mt-auto" />
                  </div>
                </div>
              </Card.Root>
            </Carousel.Item>
          {/each}
        </Carousel.Content>
      </Carousel.Root>
    {:else if trendingModsQuery.isError}
      <div
        class="mx-10 p-4 bg-destructive/10 border border-destructive/20 rounded-md"
      >
        <p class="text-destructive font-semibold text-sm">Error loading mods</p>
      </div>
    {:else if trendingModsQuery.isSuccess && trendingModsQuery.data}
      <Carousel.Root opts={{ align: "start" }} class="w-full px-10">
        <Carousel.Content class="-ml-2">
          {#each trendingModsQuery.data as mod (mod.id)}
            <Carousel.Item class="basis-full lg:basis-1/2 xl:basis-1/3">
              <Card.Root
                class="p-0 overflow-hidden hover:bg-accent/50 transition-colors"
              >
                <div class="flex h-40">
                  <div class="w-40 h-40 shrink-0 bg-muted">
                    {#if mod._links.thumbnail}
                      <img
                        src={mod._links.thumbnail}
                        alt={mod.name}
                        class="h-full w-full object-cover"
                      />
                    {:else}
                      <div
                        class="h-full w-full flex items-center justify-center"
                      >
                        <ImageOff class="h-10 w-10 text-muted-foreground/40" />
                      </div>
                    {/if}
                  </div>

                  <!-- Content Area -->
                  <div class="flex-1 p-3 flex flex-col min-w-0">
                    <div class="min-w-0 flex-1">
                      <h3
                        class="font-bold text-base leading-tight truncate mb-0.5"
                        title={mod.name}
                      >
                        {mod.name}
                      </h3>
                      <p class="text-sm text-muted-foreground/80 truncate mb-2">
                        by {mod.author}
                      </p>
                      <p
                        class="text-sm text-muted-foreground line-clamp-3 leading-snug"
                      >
                        {mod.description}
                      </p>
                    </div>

                    <div
                      class="flex items-center gap-4 text-sm font-medium pt-2"
                    >
                      <div class="flex items-center gap-1.5">
                        <Download class="h-4 w-4 text-primary" />
                        <span>{mod.downloads.toLocaleString()}</span>
                      </div>
                      <div
                        class="flex items-center gap-1.5 text-muted-foreground"
                      >
                        <Clock class="h-4 w-4" />
                        <span
                          >{new Date(mod.created_at).toLocaleDateString()}</span
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
  </section>
</div>
