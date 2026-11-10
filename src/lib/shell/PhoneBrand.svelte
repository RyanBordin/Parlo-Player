<script lang="ts">
  // The Parlo brand on the phone (spec 4.5): the symbol and "Parlo Player"
  // in the frame of the desktop sidebar's brand (spec 1.1), on the tab pages'
  // Topbar and next to ˅ in Now Playing. Decorative: the page names itself.
  import symbol from "#lib/assets/logo-symbol.png";
  import { m } from "#lib/paraglide/messages.js";
</script>

<div class="brand" aria-hidden="true">
  <img src={symbol} alt="" width="13" height="32" />
  <span>{m.app_name()}</span>
</div>

<style>
  /* The desktop brand's thin border, rounded corners and inner space, on the
     page's dark background, so it reads on Now Playing's cover color too.
     The text is a little bigger than the large text (Ryan's choice) and
     centered on the symbol. */
  .brand {
    flex: none;
    max-width: 100%;
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-4) var(--space-2) var(--space-3);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    background: var(--color-bg);
    font-size: 20px;
    line-height: 1;
    font-weight: var(--font-weight-bold);
    white-space: nowrap;
  }

  img {
    flex: none;
  }
</style>
