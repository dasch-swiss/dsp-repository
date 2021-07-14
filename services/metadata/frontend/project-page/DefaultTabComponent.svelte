<script lang="ts">
  import { onMount } from "svelte";
  import { fade } from "svelte/transition";
  import { handleSnackbar } from "../store";
  import { getText, findPersonByID, findOrganizationByID, findObjectByID } from "../functions";
  import type { TabContent, Grant, Person, Organization, Text } from "../interfaces";

  export let dataset: TabContent;

  let isAbstractExpanded: boolean;
  let abstractLinesNumber: number;

  const toggleExpand = () => {
    isAbstractExpanded = !isAbstractExpanded;
  };

  onMount(() => {
    const el = document.getElementById('abstract');
    const lineHeight = parseInt(window.getComputedStyle(el).getPropertyValue('line-height'));
    const divHeight = el.scrollHeight;
    abstractLinesNumber = divHeight / lineHeight;
    isAbstractExpanded = abstractLinesNumber > 6 ? false : true;
  });

  const copyToClipboard = () => {
    let text = document.createRange();
    text.selectNode(document.getElementById('how-to-cite'));
    window.getSelection().removeAllRanges();
    window.getSelection().addRange(text);
    document.execCommand('copy');
    window.getSelection().removeAllRanges();
    handleSnackbar.set({isSnackbar: true, message: 'Citation copied succesfully!'});
  };

  const truncateString = (s: string) => {
    const browserWidth = window.innerWidth;
    if (browserWidth < 992 && s.length > ((browserWidth - 100) / 8)) {
      return `${s.substring(0, (browserWidth - 100) / 8)}...`;
    } else if (browserWidth >= 992 && s.length > (browserWidth / 17)) {
      return `${s.substring(0, (browserWidth / 17))}...`;
    } else return s;
  };

</script>

<div id=dataset in:fade={{duration: 200}}>
  {#if dataset}
    {#if dataset?.content.alternativeTitles}
      <div>
        <span class=label>Alternative Title</span>
        <span class=data>{dataset?.content.alternativeTitles.map((t => {return getText(t)})).join(', ')}</span>
      </div>
    {/if}
  <div class="grid-wrapper">
    <div>
      <span class=label>Access</span>
      <span class=data>{dataset?.content.accessConditions}</span>
    </div>
    <div>
      <span class=label>Status</span>
      <span class=data>{dataset?.content.status}</span>
    </div>
    {#if dataset.content.dateCreated}
      <div>
        <span class=label>Date Created</span>
        <span class=data>{dataset?.content.dateCreated}</span>
      </div>
    {/if}
    {#if dataset.content.dateModified}
      <div>
        <span class=label>Date Modified</span>
        <span class=data>{dataset?.content.dateModified}</span>
      </div>
    {/if}
    <div>
      <span class=label>License</span>
      {#if Array.isArray(dataset?.content.licenses)}
        {#each dataset?.content.licenses as l}
          <a href={l.url} class="data external-link" target=_>{l.text}</a>
        {/each}
      {/if}
    </div>
    <div>
      <span class=label>Type of Data</span>
      <span class=data>{dataset?.content.typeOfData.join(', ')}</span>
    </div>

    {#if dataset?.content.documentations}
      <div style="grid-column-start: 1;grid-column-end: 3;">
        <span class=label>Additional documentation</span>
        {#each dataset?.content.documentations as d}
          {#if d.__type === "URL"}
            <a class="data external-link" href={d.url} target=_>{truncateString(d.text)}</a>
          {:else}
            <span class=data>{getText(d)}</span>
          {/if}
        {/each}
      </div>
    {/if}
  </div>

  <div class="grid-wrapper" style="grid-template-columns: repeat(1, 1fr)">
    <div>
      <span class=label>Languages</span>
      <span class=data>{dataset?.content.languages.map(l => {return getText(l)}).join(', ')}</span>
    </div>
  </div>

  {#if dataset?.content.urls}
    <div class="grid-wrapper" style="grid-template-columns: repeat(1, 1fr)">
      <div>
        <span class=label>Dataset Website</span>
        {#each dataset?.content.urls as u}
          {#if u.__type === 'URL'}
            <div><a class="data external-link" href={u.url} target=_>{truncateString(u.text)}</a></div>
          {/if}
        {/each}
      </div>
    </div>
  {/if}

  <div class="property-row">
    <span class=label style="display:inline">
      How To Cite
      <button on:click={copyToClipboard} title="copy citation to the clipboard">
         <svg class="icon" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z"></path></svg>
      </button>
    </span>
    <span id=how-to-cite class=data>{dataset?.content.howToCite}</span>
  </div>

  <div>
    <span class=label>Abstract</span>
      <div id=abstract class="data {isAbstractExpanded ? '' : 'abstract-short'}">
        {#each dataset?.content.abstracts as a}
          {#if a.__type === "URL"}
            <div><a class="data external-link" href={a.url} target=_>{truncateString(a.text)}</a></div>
          {:else}
            <div>{getText(a)}</div>
          {/if}
        {/each}
      </div>
  </div>

  {#if abstractLinesNumber > 6}
    <div on:click={toggleExpand} class=expand-button>show {isAbstractExpanded ? "less" : "more"}</div>
  {/if}

  <span class=label>Attributions</span>
  <div class="grid-wrapper">
    {#each dataset?.content.attributions as a}
    <div class="attributions data">
      <div class=role>{a.roles.join(", ")}</div>
      <!-- TODO: should this only be person or also organization? -->
        {#each [findObjectByID(a.person)] as p}
          {#if p.__type === 'Person'}
            {#if p.authorityRefs}
              <a href={p.authorityRefs[0].url} target=_ class="external-link">{p.givenNames.join(" ")} {p.familyNames.join(" ")}</a>
            {:else}
              <div>{p.givenNames.join(" ")} {p.familyNames.join(" ")}</div>
            {/if}
            {#if p.affiliation}
              {#each p.affiliation.map(o => {return findOrganizationByID(o)}) as org}
                <div>{org.name}</div>
              {/each}
            {/if}
            <div>{p.jobTitles[0]}</div>
            {#if p.emails}
              <a class=email href="mailto:{p.emails[0]}">{p.emails[0]}</a>
            {/if}
          {:else if p.__type === 'Organization'}
            {#if p.url}
              <a href={p.url.url} target=_ class="external-link">{p.name}</a>
            {/if}
            {#if p.email}
              <a class=email href="mailto:{p.email}">{p.email}</a>
            {/if}
          {/if}
        {/each}
      </div>
    {/each}
  </div>

  {/if}
</div>

<style>
  a {color: var(--lead-colour);}
  button {
    border: none;
    background-color: inherit;
    padding: 0;
    position: relative;
    top: 10px;
    color: var(--lead-colour);
    z-index: 0;
  }
  .icon {
    margin: -1rem 0 0.25rem;
  }
  .icon:hover {
    color: var(--dasch-light-violet);
  }
  .role {
    color: var(--secondary-colour);
  }
  .abstract-short {
    display: -webkit-box;
    -webkit-line-clamp: 6;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .attributions {
    padding: 10px 10px 0 0;
    line-height: 1.5;
  }
  .grid-wrapper {
    display: grid;
    grid-template-columns: repeat(1, 1fr);
  }
  @media screen and (min-width: 576px) {
    .grid-wrapper {
      grid-template-columns: repeat(2, 1fr);
    }
  }
  @media screen and (min-width: 1200px) {
    .grid-wrapper {
      grid-template-columns: repeat(3, 1fr);
    }
  }
</style>
