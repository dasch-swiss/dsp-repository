import { navigate } from 'svelte-routing';
import { writable } from 'svelte/store';
import type { PaginationData, Metadata, ProjectMetadata } from './interfaces';

export const pagination = writable({} as PaginationData);
export const pagedResults = writable(undefined as ProjectMetadata[]);
export const projectMetadata = writable(undefined as Metadata);
export const query = writable('');
export const previousRoute = writable('');
export const handleSnackbar = writable({isSnackbar: false, message: ''});

export async function getProjectsMetadata(page: number, q?: string): Promise<void> {
  // const baseUrl = process.env.BASE_URL;
  const protocol = window.location.protocol;
  const port = protocol === 'https:' ? '' : ':3000';
  const baseUrl = `${protocol}//${window.location.hostname}${port}/`;
  const baseResultsRange = [1, 9];
  let route: string;
  let currentResultsRange = baseResultsRange.map(v => v + ((page - 1) * baseResultsRange[1]));
  
  if (q) {
    query.set(q);
    route = `projects?q=${q}&_page=${page}&_limit=${baseResultsRange[1]}`;
    handleSnackbar.set({isSnackbar: true, message: `Displaying search results for query: ${q}`});
  } else {
    query.set('');
    route = `projects?_page=${page}&_limit=${baseResultsRange[1]}`;
  }

  console.log(baseUrl, route);
  navigate(`/${route}`);

  await fetch(`${baseUrl}api/v1/${route}`)
    .then(r => {
      const totalCount = parseInt(r.headers.get('X-Total-Count'));
      let totalPages = Math.floor(totalCount/baseResultsRange[1]);
      if (!Number.isInteger(totalCount/baseResultsRange[1])) {
        totalPages++;
      };
      pagination.set({currentPage: page, currentResultsRange, totalCount, totalPages});
      return r.json();
    })
    .then(data => {pagedResults.set(data), console.log(data)})
}
