<script lang="ts">
  import { QueryClient, QueryClientProvider } from "@tanstack/svelte-query";
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";

  import App from "./App.svelte";
  import type { CdiStatusDto } from "./lib/api/bindings";

  const queryClient = new QueryClient({
    defaultOptions: {
      queries: {
        retry: 1,
        staleTime: 10_000,
        refetchOnWindowFocus: false,
      },
    },
  });

  onMount(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;

    void listen<CdiStatusDto>("cdi-status", (event) => {
      queryClient.setQueryData<CdiStatusDto>(["cdi", "status"], event.payload);
    }).then((dispose) => {
      if (disposed) {
        dispose();
        return;
      }
      unlisten = dispose;
      // O refresh de startup pode emitir antes do listener existir; recarregar
      // o status uma vez cobre o evento perdido sem recorrer a polling.
      void queryClient.invalidateQueries({ queryKey: ["cdi", "status"] });
    });

    return () => {
      disposed = true;
      unlisten?.();
    };
  });
</script>

<QueryClientProvider client={queryClient}>
  <App />
</QueryClientProvider>
