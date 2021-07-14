<script lang='ts'>
  import { tick } from 'svelte';
  import { projectMetadata, handleSnackbar, previousRoute } from '../store';
  import type {Metadata} from "../interfaces";
  import ProjectWidget from './ProjectWidget.svelte';
  import DownloadWidget from './DownloadWidget.svelte';
  import Tab from './Tab.svelte';
  import { fade } from 'svelte/transition';
  import Snackbar from '../Snackbar.svelte';
  import { getText } from "../functions";

  const mobileResolution = window.innerWidth < 992;
  
  let isDescriptionExpanded: boolean;
  let descriptionLinesNumber: number;
  let arePublicationsExpanded: boolean;

  const getProjectMetadata = async () => {
    // TODO: can this be cleaned up?
    const protocol = window.location.protocol;
    const port = protocol === 'https:' ? '' : ':3000';
    const baseUrl = `${protocol}//${window.location.hostname}${port}/`;
    const projectID = window.location.pathname.split("/")[2];
    
    // const res = await fetch(`${process.env.BASE_URL}projects/${params.id}`);
    const res = await fetch(`${baseUrl}api/v1/projects/${projectID}`);
    const metadata: Metadata = await res.json();
    projectMetadata.set(metadata);

    // const project = $currentProjectMetadata.project
    // currentProject.set(project);
    document.title = metadata.project.name;

    // datasets = $projectMetadata.datasets

    // datasets.forEach(d => tabs.push({
    //   label: d.title,
    //   value: datasets.indexOf(d),
    //   content: d
    // }));

    await tick();
    getDivHeight();
  };

  const toggleDescriptionExpand = () => {
    isDescriptionExpanded = !isDescriptionExpanded;
    !isDescriptionExpanded ? window.scrollTo(0,0) : null;
  };

  const togglePublicationExpand = () => {
    arePublicationsExpanded = !arePublicationsExpanded;
    !arePublicationsExpanded ? window.scrollTo(0,300) : null;
  };

  const getDivHeight = () => {
    const el = document.getElementById('description');
    const lineHeight = parseInt(window.getComputedStyle(el).getPropertyValue('line-height'));
    const divHeight = el.scrollHeight;
    descriptionLinesNumber = divHeight / lineHeight;
    isDescriptionExpanded = descriptionLinesNumber > 6 ? false : true;
  };
</script>

{#if $handleSnackbar.isSnackbar}
  <div>
    <svelte:component this={Snackbar} />
  </div>
{/if}

<div class="container" in:fade={{duration: 200}}>
  {#if mobileResolution}
    <button on:click={() => history.back()} class=goback-button title="go back to the projects list" disabled={!$previousRoute && window.history.length <= 2}>
      <svg class=icon fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 19l-7-7m0 0l7-7m-7 7h18" />
      </svg>
      <span class=button-label>Go Back</span>
    </button>
  {/if}
  <div class="row" style="flex-wrap: wrap;">
    <h1 class="title top-heading">
      {$projectMetadata?.project.name}
    </h1>
    {#if $projectMetadata?.project.alternativeNames}
    <div class="row">
      <h4 class="title new-title">
        Also known as:&nbsp;
        <span style="color:var(--secondary-colour)">{$projectMetadata?.project.alternativeNames.map(t => {return getText(t)}).join(', ')}</span>
      </h4>
    </div>
    {/if}
  </div>
  <div class="row">
    <div class="column-left">
      <div class="property-row">
        <span class="label new-subtitle">Description</span>
        <div id=description class="data new-text {isDescriptionExpanded ? '' : 'description-short'}">{getText($projectMetadata?.project.description)}</div>
      </div>
      <!-- TODO: if accepted and reused consder move it to separate component -->
      {#if descriptionLinesNumber > 6}
        <div on:click={toggleDescriptionExpand} class=expand-button>show {isDescriptionExpanded ? "less" : "more"}</div>
      {/if}

      {#if $projectMetadata?.project.publications && Array.isArray($projectMetadata?.project.publications)}
        <div class="property-row">
          <span class="label new-subtitle">Publications</span>
            {#each $projectMetadata?.project.publications as p, i}
              {#if i > 1}
                <span class="{arePublicationsExpanded ? "data new-text" : "hidden"}">{p}</span>
              {:else}
                <span class="data new-text">{p}</span>
              {/if}
            {/each}
        </div>

        {#if $projectMetadata?.project.publications.length > 2}
          <div on:click={togglePublicationExpand} class=expand-button>show {arePublicationsExpanded ? "less" : "more"}</div>
        {/if}

      {/if}

      {#await getProjectMetadata() then go}
        <div class="tabs">
          <Tab datasets={$projectMetadata?.datasets} />
        </div>
      {/await}

      {#if !mobileResolution}
        <button on:click={() => window.scrollTo({top: 0, left: 0, behavior: 'smooth'})} class=gototop-button title="Get back to the top">
          <svg class=icon fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 10l7-7m0 0l7 7m-7-7v18" />
          </svg>
        </button>
      {/if}

    </div>
    <div class="column-right">
      {#if !mobileResolution}
        <button on:click={() => history.back()} class=goback-button title="go back to the projects list" disabled={!$previousRoute && window.history.length <= 2}>
          <svg class=icon fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 19l-7-7m0 0l7-7m-7 7h18" />
          </svg>
          <span class=button-label>Go Back</span>
        </button>
      {/if}

      <div class=widget>
        <ProjectWidget />
      </div>

      <!-- TODO: temp disabled download widget -->
      <!-- <div class=widget>
        <DownloadWidget />
      </div> -->

      {#if mobileResolution}
        <button on:click={() => {window.scrollTo({top: 0, left: 0, behavior: 'smooth'})}} class="gototop-button m-hidden" title="Get back to the top">
          <svg class=icon fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 10l7-7m0 0l7 7m-7-7v18" />
          </svg>
        </button>
      {/if}
    </div>
  </div>
</div>

<style>
  button {
    color: var(--lead-colour);
    box-shadow: var(--shadow-1);
    border: 1px solid #cdcdcd;
    border-radius: 0.25rem;
  }
  button.goback-button {
    margin-top: 10px;
    width: 100%;
    font-size: 1rem;
    font-family: robotobold;
    text-align: left;
    margin-bottom: 6px;
    padding: 10px 10px 8px;
  }
  .button-label {
    position: relative;
    bottom: 10px;
  }
  button.gototop-button {
    display: inline-block;
    vertical-align: middle;
    background-color: var(--dasch-grey-3);
    padding: 10px;
    width: 3.5rem;
    height: 3.5rem;
  }
  button.gototop-button:hover,
  button.goback-button:hover {
    color: #fff;
    background-color: var(--lead-colour);
  }
  .container {
    padding: 0 10px;
    display: block;
    max-width: 1200px;
  }
  .title {
    display: flex;
    flex-direction: row;
    flex-basis: 100%;
    margin-bottom: 0;
    padding: 0 20px;
  }
  .column-left, .column-right {
    display: flex;
    flex-direction: column;
    flex-basis: 100%;
    flex: 2;
    padding: 0 5px;
    height: fit-content;
  }
  .column-right {
    flex: 1;
  }
  .description-short {
    display: -webkit-box;
    -webkit-line-clamp: 6;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .widget {
    border: 1px solid #cdcdcd;
    border-radius: 3px;
    background-color: var(--dasch-grey-3);
    margin-bottom: 6px;
    padding: 0 10px 10px;
    box-shadow: var(--shadow-1);
  }
  @supports (-moz-appearance:none) {
    button.gototop-button {margin-bottom: 40px;} 
  }
  @media screen and (min-width: 992px) {
    .container {padding: 0 40px}
    .column-left, .column-right {padding: 20px;}
    .column-left {min-width: 52vw;}
    .column-right {min-width: 30vw;}
    .row {flex-direction: row;}
  }
  @media screen and (min-width: 1200px) {
    .column-left {min-width: 688px;}
    .column-right {min-width: 352px;}
  }
</style>
