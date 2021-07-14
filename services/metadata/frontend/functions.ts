
import {get} from "svelte/store";
import {projectMetadata as ProjectMetadata} from "./store";
import type {Grant, Person, Organization, Text} from "./interfaces";

export function findPersonByID(id:string): Person {
  let persons = get(ProjectMetadata).persons;
  if (persons && persons.length > 0) {
    return persons.find(o => o.__id === id);
  }
}

export function findOrganizationByID(id:string): Organization {
  return get(ProjectMetadata).organizations.find(o => o.__id === id);
}

export function findGrantByID(id:string): Grant {
  return get(ProjectMetadata).grants.find(o => o.__id === id);
}

export function findObjectByID(id:string): Grant | Person | Organization {
  let o: Grant | Person | Organization;
  o = findPersonByID(id);
  if (o) return o;
  o = findOrganizationByID(id);
  if (o) return o;
  o = findGrantByID(id);
  if (o) return o;
}

export function getText(text: Text, lang?: string) {
  if (!text) {
    return ""
  }

  let langs = Object.keys(text);

  if (langs.length === 0) {
    return ""
  } else if (lang && langs.includes(lang)) {
    return text[lang]
  } else if (langs.includes('en')) {
    return text['en']
  } else {
    return text[langs[0]]
  }
}
