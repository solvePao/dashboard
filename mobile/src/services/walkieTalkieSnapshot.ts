import type { WalkieCall, WalkieSnapshot } from "../types/walkieTalkie";

export function sameFields<T extends object>(a: T, b: T): boolean {
  if (a === b) return true;
  const keys = Object.keys(a) as (keyof T)[];
  return keys.length === Object.keys(b).length && keys.every(key => a[key] === b[key]);
}

export function sameList<T>(a: T[], b: T[], equal: (left: T, right: T) => boolean): boolean {
  return a === b || (a.length === b.length && a.every((item, i) => equal(item, b[i])));
}

function sameCall(a: WalkieCall | null | undefined, b: WalkieCall | null | undefined): boolean {
  if (a === b) return true;
  if (!a || !b) return false;
  const { participants, declined, ...fields } = a;
  const { participants: nextParticipants, declined: nextDeclined, ...nextFields } = b;
  return sameFields(fields, nextFields)
    && sameList(participants, nextParticipants, Object.is)
    && sameList(declined, nextDeclined, Object.is);
}

export function mergeSnapshot(previous: WalkieSnapshot | undefined, next: WalkieSnapshot): WalkieSnapshot {
  if (!previous) return next;
  const members = sameList(previous.members, next.members, sameFields) ? previous.members : next.members;
  const clients = sameList(previous.clients, next.clients, sameFields) ? previous.clients : next.clients;
  const call = sameCall(previous.call, next.call) ? previous.call : next.call;
  if (previous.group_name === next.group_name && members === previous.members
      && clients === previous.clients && call === previous.call) return previous;
  return { ...next, members, clients, call };
}
