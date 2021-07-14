<script lang='ts'>
  import Tile from './Tile.svelte';
  import Category from './Category.svelte';
  import { onMount } from 'svelte';
  import Pagination from './Pagination.svelte';
  import { getProjectsMetadata, handleSnackbar, pagedResults } from '../store';
  import { fade } from 'svelte/transition';
  import Snackbar from '../Snackbar.svelte';

  let message = 'Loading...';

  $: document.title = 'DaSCH Metadata Browser';

  setTimeout(() => {
    const noData = 'No data retrived. Please check the connection and retry.';
    const noProject = 'No projects found.'
      message = $pagedResults && $pagedResults.length ? noData : noProject;
    }, 3000);
  
  onMount(async () => {
    // TODO: add preventing go back button to get back out of domain

    // get searchUri and 
    const searchUri = window.location.search;
    const params = new URLSearchParams(searchUri);
    const page = Number(params.get('_page'));
    const query = params.get('q');
    console.log(searchUri, query, page, params.get('_limit'),);

    // load projects
    if (!$pagedResults && !searchUri) {
      // first page on main page arrival
      await getProjectsMetadata(1);
    } else {
      // preserved on rehresh or manually entered query
      await getProjectsMetadata(page, query);
    }
  });
</script>

<nav>
  <div class="category-container hidden m-inline-block">
    <Category />
  </div>
</nav>

{#if $handleSnackbar.isSnackbar}
  <div>
    <svelte:component this={Snackbar} />
  </div>
{/if}

<main in:fade="{{duration: 200}}">
  <div class=tile-container>
    {#if $pagedResults && $pagedResults.length}
      {#each $pagedResults as project}
        <!-- TODO: remove actual metadata content from BE response on /projects endpoint, and see what that would break -->
        <Tile metadata={project}/>
      {/each}
    {:else}
      <p>{message}</p>
    {/if}
  </div>
  {#if $pagedResults && $pagedResults.length}
    <Pagination />
  {/if}
</main>

<style>
* {
  box-sizing: border-box;
}
nav, main {
  width: 100%;
  min-height: auto;
  padding: 10px 24px;
}
nav {
  flex: 0 0 20%;
  display: flex;
  justify-content: flex-end;
  padding: 0;
  /* TODO: temp hidden faceated search */
  display: none;
}
.category-container {
  padding-top: 45px;
  max-width: 210px;
}
main {
  width: 100%;
  align-items: center;
  justify-content: center;
}
.tile-container {
  padding: 10px 5px;
  display: flex;
  flex-flow: row wrap;
  justify-content: center;
  max-width: 1200px;
}
@media screen and (min-width: 992px) {
  nav, main {
    /* min-height: 950px; */
  }
  nav {
    padding: 10px;
  }
  .tile-container {
    padding: 40px 0;
    min-width: 742px;
  }
}
@media screen and (min-width: 1200px) {
  .tile-container {
    min-width: 940px;
  }
}
@media screen and (min-width: 768px) and (max-width: 1023px) { }
@media screen and (min-width: 1024px) and (max-width: 1365px) { }
@media screen and (min-width: 1366px) {}
</style>
