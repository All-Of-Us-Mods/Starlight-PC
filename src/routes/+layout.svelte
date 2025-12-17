<script lang="ts">
    import "../app.css";
    import { browser } from "$app/environment";
    import { QueryClient, QueryClientProvider } from "@tanstack/svelte-query";
    import { ModeWatcher } from "mode-watcher";

    const queryClient = new QueryClient({
        defaultOptions: {
            queries: {
                enabled: browser,
                staleTime: 1000 * 60 * 5, // 5 minutes - data stays fresh
                gcTime: 1000 * 60 * 10, // 10 minutes - keep in cache
            },
        },
    });
    let { children } = $props();
</script>

<ModeWatcher />
<QueryClientProvider client={queryClient}>
    {@render children?.()}
</QueryClientProvider>
