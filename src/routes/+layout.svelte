<script lang="ts">
    import "../app.css";
    import { browser } from "$app/environment";
    import { QueryClient, QueryClientProvider } from "@tanstack/svelte-query";
    import { ModeWatcher } from "mode-watcher";

    const queryClient = new QueryClient({
        defaultOptions: {
            queries: {
                enabled: browser,
            },
        },
    });
    let { children } = $props();
</script>

<ModeWatcher />
<QueryClientProvider client={queryClient}>
    {@render children?.()}
</QueryClientProvider>
