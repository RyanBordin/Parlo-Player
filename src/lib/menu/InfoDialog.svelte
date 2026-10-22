<script lang="ts">
  // "File information" of a song and "Information" of an album or artist
  // (spec 8.1), from the local cache. Fields the server did not send are
  // left out.
  import { getAlbumInfo, getArtistInfo, getSongInfo } from "#lib/api.ts";
  import { describeError, type ErrorMessage } from "#lib/errors.ts";
  import { formatBytes, formatDuration, formatNumber, formatTotalDuration } from "#lib/format.ts";
  import { m } from "#lib/paraglide/messages.js";

  let { target, onclose }: { target: { kind: "song" | "album" | "artist"; id: string }; onclose: () => void } = $props();

  type Info = { title: string; subtitle: string | null; rows: [string, string][]; text: string | null };

  const id = $props.id();
  let dialog = $state<HTMLDialogElement>();
  /** `undefined` while reading, `null` if no longer in the library. */
  let info = $state<Info | null | undefined>(undefined);
  let error = $state<ErrorMessage | null>(null);

  $effect(() => {
    dialog?.showModal();
  });

  $effect(() => {
    read(target).then(
      (result) => (info = result),
      (e) => (error = describeError(e)),
    );
  });

  function rows(pairs: [string, string | null | undefined | false][]): [string, string][] {
    return pairs.filter((pair): pair is [string, string] => typeof pair[1] === "string" && pair[1] !== "");
  }

  function khz(hz: number): string {
    return m.info_khz({ value: formatNumber(hz / 1000) });
  }

  async function read(item: typeof target): Promise<Info | null> {
    if (item.kind === "song") {
      const song = await getSongInfo(item.id);
      if (!song) return null;
      return {
        title: song.title,
        subtitle: [song.artist, song.album].filter(Boolean).join(" • ") || null,
        rows: rows([
          [m.info_format(), song.suffix?.toUpperCase() ?? song.contentType],
          [m.info_bit_rate(), song.bitRate != null && m.tracklist_kbps({ value: song.bitRate })],
          [m.info_sampling_rate(), song.samplingRate != null && khz(song.samplingRate)],
          [m.info_bit_depth(), song.bitDepth != null && m.info_bits({ value: song.bitDepth })],
          [m.info_channels(), song.channelCount != null && formatNumber(song.channelCount)],
          [m.info_size(), song.size != null && formatBytes(song.size)],
          [m.info_duration(), song.duration != null && formatDuration(song.duration)],
          [m.info_path(), song.path],
        ]),
        text: null,
      };
    }
    if (item.kind === "album") {
      const album = await getAlbumInfo(item.id);
      if (!album) return null;
      return {
        title: album.name,
        subtitle: album.artist,
        rows: rows([
          [m.info_year(), album.year != null && String(album.year)],
          [m.info_label(), album.labels.join(", ")],
          [m.info_genre(), album.genres.join(", ")],
          [m.info_format(), album.formats.map((f) => f.toUpperCase()).join(", ")],
          [m.info_songs(), formatNumber(album.songCount)],
          [m.info_duration(), album.duration != null && formatTotalDuration(album.duration)],
        ]),
        text: null,
      };
    }
    const artist = await getArtistInfo(item.id);
    if (!artist) return null;
    return {
      title: artist.name,
      subtitle: null,
      rows: rows([
        [m.info_albums(), formatNumber(artist.albumCount)],
        [m.info_songs(), formatNumber(artist.songCount)],
      ]),
      text: artist.biography,
    };
  }

  let heading = $derived(
    target.kind === "song" ? m.menu_file_info() : target.kind === "album" ? m.info_title_album() : m.info_title_artist(),
  );
</script>

<dialog bind:this={dialog} {onclose} aria-labelledby={`${id}-title`}>
  <h2 id={`${id}-title`}>{heading}</h2>
  {#if error}
    <p class="error">{error.message}</p>
  {:else if info === null}
    <p class="muted">{m.info_not_found()}</p>
  {:else if info}
    <p class="name">{info.title}</p>
    {#if info.subtitle}<p class="muted">{info.subtitle}</p>{/if}
    <dl>
      {#each info.rows as [label, value] (label)}
        <div>
          <dt>{label}</dt>
          <dd>{value}</dd>
        </div>
      {/each}
    </dl>
    {#if info.text}
      <h3>{m.info_biography()}</h3>
      <p class="text">{info.text}</p>
    {/if}
  {/if}
  <div class="buttons">
    <button type="button" class="button button-primary" onclick={() => dialog?.close()}>{m.info_close()}</button>
  </div>
</dialog>

<style>
  dialog {
    width: min(520px, calc(100vw - 48px));
    padding: var(--space-5);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    background: var(--color-surface);
    color: var(--color-text);
    box-shadow: var(--shadow-dialog);
  }

  dialog::backdrop {
    background: var(--color-backdrop);
  }

  h2 {
    margin-bottom: var(--space-3);
    font-size: var(--font-size-lg);
    font-weight: var(--font-weight-bold);
  }

  h3 {
    margin-top: var(--space-4);
    color: var(--color-text-muted);
    font-size: var(--font-size-sm);
    font-weight: var(--font-weight-semibold);
  }

  .name {
    font-weight: var(--font-weight-semibold);
  }

  .muted {
    color: var(--color-text-muted);
    font-size: 14px;
  }

  dl {
    display: grid;
    grid-template-columns: max-content minmax(0, 1fr);
    gap: var(--space-2) var(--space-4);
    margin: var(--space-4) 0 0;
    font-size: 14px;
  }

  dl > div {
    display: contents;
  }

  dt {
    color: var(--color-text-muted);
  }

  dd {
    margin: 0;
    overflow-wrap: anywhere;
    user-select: text;
  }

  .text {
    max-height: 200px;
    margin-top: var(--space-2);
    overflow-y: auto;
    font-size: 14px;
    line-height: 1.5;
    white-space: pre-line;
  }

  .error {
    color: var(--color-error);
    font-size: var(--font-size-sm);
  }

  .buttons {
    display: flex;
    justify-content: flex-end;
    margin-top: var(--space-5);
  }
</style>
