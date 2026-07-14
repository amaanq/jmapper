# DAV integration status

`dav-sync` is wired into `jmapper` and `jmap-server`.

- `jmapper` persists configured CalDAV/CardDAV endpoints, supervises one DAV
  task per configured account, performs initial and periodic sync, and
  reconciles tasks on SIGHUP.
- `AppState` exposes endpoint-specific handles and availability. The Session
  resource advertises calendar/contact capabilities only when the
  authenticated account has the matching endpoint.
- `Calendar`, `CalendarEvent`, `AddressBook`, and `ContactCard` methods read
  the PostgreSQL cache. Resource writes are remote-first through `DavHandle`;
  collection mutation is read-only until MKCOL/PROPPATCH support exists.
- Four independent modseq streams back `/changes`; expiring query snapshots
  back resource `/queryChanges`.

## Deliberate protocol limits

- Date filters expand recurrence rules, RDATE/EXDATE values, and overrides
  through calcard. Query results do not mint synthetic occurrence ids, so the
  capability still advertises `maxExpandedQueryDuration = PT0S`.
- Generated iCalendar references IANA TZIDs without synthesizing VTIMEZONE.
- There is no iTIP/iMIP scheduling, WebDAV ACL/sharing model, or managed
  attachment support. CalendarEventNotification is therefore an empty
  collection. A request with `sendSchedulingMessages = true` fails each
  requested mutation without changing remote or cached state.
- `utcStart` and `utcEnd` are computed for reads but are not accepted as write
  inputs. CalendarEvent/parse is not advertised or implemented.
- A malformed remote resource is skipped and reported; a malformed
  replacement tombstones a previously valid cached representation so stale
  data is not served. The collection token advances only after the complete
  sync round commits.
- Unknown iCalendar/vCard properties are preserved through
  `jmapper.rs:icalProps` and RFC 9555 `vCardProps`.
- A single DAV response is capped at 64 MiB, redirects never cross origins or
  carry URL-embedded credentials, and HTTP 303 responses are not replayed as
  DAV writes.
