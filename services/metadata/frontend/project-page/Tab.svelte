<script lang="ts">
  import DefaultTabComponent from "./DefaultTabComponent.svelte";
  import type { Dataset, TabContent } from "../interfaces";

  export let datasets = [] as Dataset[];
  // export let tabs = [] as any[];
  export let activeTabValue = 0;

  // TODO: maybe we can even drop the TabContent interface completely?
  let tabs = [] as TabContent[]
  datasets.forEach(d => tabs.push({
    label: d.title,
    value: datasets.indexOf(d),
    content: d
  }));

  const handleTabsBrowsing = (tabValue: number) => () => (activeTabValue = tabValue);
</script>

<ul>
  {#each tabs as tab}
    <li class={activeTabValue === tab.value ? 'active' : ''}>
      {#if tabs.length > 1 && activeTabValue !== tab.value}
        <span on:click={handleTabsBrowsing(tab.value)} title={tab.label}>{`${tab.label.substring(0,12)}...`}</span>
      {:else}
        <span on:click={handleTabsBrowsing(tab.value)}>{tab.label}</span>
      {/if}
    </li>
  {/each}
</ul>
{#each tabs as tab}
	{#if activeTabValue === tab.value}
    <div class=box>
      <svelte:component this={DefaultTabComponent} dataset={tab} />
    </div>
	{/if}
{/each}

<style>
  .box {
    margin-bottom: 10px;
    padding: 0 10px;
    border: 1px solid #dee2e6;
    border-radius: 0 0 .5rem .5rem;
    border-top: 0;
    overflow-wrap: break-word;
    box-shadow: var(--shadow-2);
  }
  ul {
    display: flex;
    flex-wrap: wrap;
    padding-left: 0;
    margin-bottom: 0;
    list-style: none;
    border-bottom: 1px solid #dee2e6;
  }
  li {
    margin-bottom: -1px;
  }
  span {
    border: 1px solid #e9ecef;
    border-top-left-radius: 0.25rem;
    border-top-right-radius: 0.25rem;
    display: block;
    padding: 0.5rem 1rem;
    cursor: pointer;
  }
  span:hover {
    background-color: var(--dasch-light-violet);
  }
  li.active > span {
    color: #fff;
    background-color: var(--lead-colour);
    border-color: #dee2e6 #dee2e6 #fff;
  }
</style>
