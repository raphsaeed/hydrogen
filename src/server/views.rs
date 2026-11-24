//! Carbon Warm-Paper Light-Theme HTML views for `GET /` (Workspace Landing Page)
//! and `GET /products/{product}` (Product Sub-Cards Explorer Page) (`Phase 3A.2`).
//!
//! Strictly mirrors `docs/carbon-visual-theme.md`:
//! - Canvas `#FDFBF7` (Warm Ivory Paper), secondary `#FDFCF9`,
//! - Chrome bar `#F2ECE1` (border `#DBD2C2`, pills `#FCF9F2`),
//! - Pure White cards `#FFFFFF` (border `#DEE4EC`),
//! - Primary charcoal ink `#1F2421`, muted label ink `#68645B`,
//! - Canonical 12-subject muted mineral accents (`#5B7DB1`, `#4D8B7A`, `#7A6BA8`, `#A8647A`, `#568A9E`, ...).

use std::fmt::Write as _;

use crate::ingest::escape_html;
use crate::server::scanner::{ProductOverview, SubProductOverview, WorkspaceOverview};

const SHARED_WARM_PAPER_CSS: &str = r##"
    :root {
      --paper-bg: #FDFBF7;
      --board-bg: #FDFCF9;
      --chrome-bg: #F2ECE1;
      --chrome-border: #DBD2C2;
      --pill-bg: #FCF9F2;
      --card-bg: #FFFFFF;
      --card-border: #DEE4EC;
      --ink-primary: #1F2421;
      --ink-muted: #68645B;
      --blue: #2563EB;
      --blue-bg: #EFF6FF;
      --blue-border: #BAD4F7;
      --emerald: #15803D;
      --emerald-bg: #F0FDF4;
      --emerald-border: #B2E8C6;
      --rose: #DC2626;
      --rose-bg: #FFF1F2;
      --rose-border: #FBABB8;
      --amber: #B45309;
      --amber-bg: #FEF9C3;
      --amber-border: #CA8A04;
      --violet: #6D5CAE;
      --violet-bg: #F5F3FF;
      --violet-border: #DDD6FE;
    }
    * { box-sizing: border-box; }
    html { scrollbar-gutter: stable; }
    body {
      margin: 0;
      background: var(--paper-bg);
      color: var(--ink-primary);
      font-family: "Lato", -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
      line-height: 1.5;
    }
    a { color: var(--blue); text-decoration: none; }
    a:hover { text-decoration: underline; }
    header.topbar {
      width: 100%;
      height: 56px;
      background: var(--chrome-bg);
      border-bottom: 1px solid var(--chrome-border);
      padding: 0 28px;
      display: grid;
      grid-template-columns: 1fr auto 1fr;
      align-items: center;
      gap: 16px;
    }
    .topbar-left {
      justify-self: start;
      display: flex;
      align-items: center;
    }
    .topbar-center {
      justify-self: center;
      display: flex;
      align-items: center;
      justify-content: center;
      text-align: center;
    }
    .topbar-center h1 {
      margin: 0;
      font-size: 1.16rem;
      font-weight: 700;
      letter-spacing: -0.01em;
      color: var(--ink-primary);
      white-space: nowrap;
    }
    .topbar-right {
      justify-self: end;
      display: flex;
      align-items: center;
    }
    .hydrogen-mark {
      height: 36px;
      width: auto;
      background: transparent;
      object-fit: contain;
      display: block;
    }
    .page-hero {
      width: fit-content;
      max-width: 100%;
      margin: 0 auto 16px;
      padding: 5px 12px;
      background: var(--board-bg);
      border: 1px solid var(--chrome-border);
      border-radius: 999px;
      display: flex;
      flex-wrap: wrap;
      align-items: center;
      justify-content: center;
      gap: 6px;
    }
    .hero-sep {
      color: var(--chrome-border);
      font-size: 0.8rem;
      user-select: none;
    }
    img.brand-logo {
      height: 20px;
      width: auto;
      background: transparent;
      display: block;
      object-fit: contain;
    }
    .breadcrumbs {
      display: inline-flex;
      align-items: center;
      justify-content: center;
      flex-wrap: wrap;
      gap: 4px;
      font-size: 0.75rem;
      color: var(--ink-muted);
    }
    .crumb-pill {
      background: var(--pill-bg);
      border: 1px solid var(--chrome-border);
      border-radius: 5px;
      padding: 1px 6px;
      font-size: 0.74rem;
      font-weight: 600;
      color: var(--ink-primary);
    }
    a.crumb-pill:hover {
      border-color: var(--blue);
      color: var(--blue);
      text-decoration: none;
    }
    .crumb-active {
      font-weight: 700;
      font-size: 0.75rem;
      color: var(--ink-primary);
    }
    .crumb-sep { color: var(--ink-muted); font-size: 0.7rem; }
    .kpi-strip {
      display: inline-flex;
      flex-wrap: wrap;
      align-items: center;
      justify-content: center;
      gap: 5px;
    }
    .kpi-pill {
      background: var(--pill-bg);
      border: 1px solid var(--chrome-border);
      border-radius: 999px;
      padding: 2px 8px;
      font-size: 0.73rem;
      font-weight: 600;
      line-height: 1.35;
    }
    .kpi-pill.ok {
      background: var(--emerald-bg);
      border-color: var(--emerald-border);
      color: var(--emerald);
    }
    .kpi-pill.alert {
      background: var(--rose-bg);
      border-color: var(--rose-border);
      color: #881337;
    }
    main {
      max-width: 1380px;
      margin: 0 auto;
      padding: 14px 28px 52px;
    }
    .section-head {
      display: flex;
      flex-direction: column;
      align-items: center;
      text-align: center;
      gap: 4px;
      border-bottom: 1px solid var(--chrome-border);
      padding-bottom: 10px;
      margin-bottom: 20px;
    }
    .section-head h2 {
      margin: 0;
      font-size: 1.08rem;
      color: var(--ink-primary);
    }
    .muted {
      color: var(--ink-muted);
      font-size: 0.83rem;
    }
    code {
      font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
      font-size: 0.81rem;
      background: #F8FAFC;
      padding: 1px 5px;
      border-radius: 4px;
      border: 1px solid #E2E8F0;
    }
    .card-grid {
      display: grid;
      grid-template-columns: repeat(auto-fill, minmax(360px, 1fr));
      gap: 18px;
    }
    .explorer-card {
      position: relative;
      cursor: pointer;
      background: var(--card-bg);
      border: 1px solid var(--card-border);
      border-radius: 12px;
      padding: 18px 20px;
      box-shadow: 0 1px 3px rgba(31, 36, 33, 0.05);
      display: flex;
      flex-direction: column;
      gap: 12px;
      transition: border-color 0.14s ease, box-shadow 0.14s ease, transform 0.14s ease;
    }
    .explorer-card:hover {
      border-color: var(--blue-border);
      box-shadow: 0 6px 18px rgba(31, 36, 33, 0.08);
      transform: translateY(-1px);
    }
    .explorer-card.subproduct-card {
      border-left-width: 6px;
    }
    .card-header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 12px;
    }
    .card-brand-center {
      display: flex;
      flex-direction: column;
      align-items: center;
      text-align: center;
      gap: 6px;
    }
    .product-logo-box {
      position: relative;
      z-index: 2;
      height: 38px;
      background: transparent;
      display: inline-flex;
      align-items: center;
      justify-content: center;
      flex-shrink: 0;
    }
    .product-logo-box img {
      max-height: 36px;
      max-width: 130px;
      width: auto;
      height: auto;
      display: block;
      object-fit: contain;
    }
    .card-title-link {
      color: var(--ink-primary);
      font-weight: 700;
      font-size: 1.08rem;
    }
    .card-title-link::after {
      content: "";
      position: absolute;
      inset: 0;
      z-index: 1;
      border-radius: inherit;
    }
    .card-title-link:hover {
      color: var(--blue);
      text-decoration: none;
    }
    .badge {
      display: inline-block;
      padding: 2px 9px;
      border-radius: 999px;
      font-size: 0.75rem;
      font-weight: 600;
      background: #F1F5F9;
      color: #475569;
      border: 1px solid #CBD5E1;
      white-space: nowrap;
    }
    .badge.ok { background: var(--emerald-bg); color: var(--emerald); border-color: var(--emerald-border); }
    .badge.progress { background: var(--blue-bg); color: var(--blue); border-color: var(--blue-border); }
    .badge.warn { background: var(--amber-bg); color: var(--amber); border-color: var(--amber-border); }
    .badge.danger { background: var(--rose-bg); color: #881337; border-color: var(--rose-border); }
    .badge.ai { background: var(--violet-bg); color: var(--violet); border-color: var(--violet-border); }
    .chip-row {
      display: flex;
      flex-wrap: wrap;
      gap: 7px;
    }
    a.mineral-chip {
      position: relative;
      z-index: 2;
      display: inline-flex;
      align-items: center;
      gap: 6px;
      background: var(--board-bg);
      border: 1px solid var(--card-border);
      border-left-width: 4px;
      border-radius: 8px;
      padding: 3px 10px;
      font-size: 0.8rem;
      font-weight: 600;
      color: var(--ink-primary);
    }
    a.mineral-chip:hover {
      background: var(--blue-bg);
      text-decoration: none;
    }
    .progress-block {
      display: flex;
      flex-direction: column;
      gap: 5px;
    }
    .progress-label-row {
      display: flex;
      justify-content: space-between;
      align-items: baseline;
      font-size: 0.83rem;
    }
    .progress-bar-track {
      width: 100%;
      height: 10px;
      background: #EDF2F7;
      border-radius: 999px;
      overflow: hidden;
    }
    .progress-bar-fill {
      height: 100%;
      background: var(--emerald);
      border-radius: 999px;
    }
    .metrics-grid {
      display: grid;
      grid-template-columns: repeat(2, minmax(0, 1fr));
      gap: 5px 14px;
      font-size: 0.84rem;
      color: var(--ink-muted);
      background: var(--board-bg);
      border: 1px solid #EEF2F6;
      border-radius: 8px;
      padding: 10px 12px;
    }
    .metrics-grid strong {
      color: var(--ink-primary);
    }
    .card-footer {
      display: flex;
      flex-wrap: wrap;
      align-items: center;
      justify-content: space-between;
      gap: 8px;
      margin-top: auto;
      padding-top: 4px;
    }
    .card-footer form,
    .card-footer a,
    .card-footer button {
      position: relative;
      z-index: 2;
    }
    .btn-primary {
      display: inline-block;
      background: var(--blue);
      color: #FFFFFF;
      border: 1px solid #1D4ED8;
      border-radius: 999px;
      padding: 4px 11px;
      font-size: 0.76rem;
      font-weight: 700;
      cursor: pointer;
    }
    .btn-primary:hover {
      background: #1D4ED8;
      color: #FFFFFF;
      text-decoration: none;
    }
    .btn-secondary {
      display: inline-block;
      background: var(--pill-bg);
      color: var(--ink-primary);
      border: 1px solid var(--chrome-border);
      border-radius: 999px;
      padding: 4px 10px;
      font-size: 0.75rem;
      font-weight: 600;
      cursor: pointer;
    }
    .btn-secondary:hover {
      border-color: var(--blue);
      color: var(--blue);
      background: var(--blue-bg);
      text-decoration: none;
    }
    .empty-card {
      background: var(--card-bg);
      border: 1px dashed var(--chrome-border);
      border-radius: 12px;
      padding: 28px;
      text-align: center;
      color: var(--ink-muted);
    }
"##;

fn compute_progress_pct(verified: usize, expected: usize) -> f64 {
    if expected == 0 {
        if verified > 0 { 100.0 } else { 0.0 }
    } else {
        #[allow(clippy::cast_precision_loss)]
        let pct = (verified as f64 / expected as f64) * 100.0;
        pct.clamp(0.0, 100.0)
    }
}

/// Render `GET /` — Carbon Warm-Paper Hydrogen Workspace Landing Page.
pub fn render_workspace_landing_html(overview: &WorkspaceOverview) -> String {
    let mut html = String::with_capacity(16_384);
    html.push_str(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Hydrogen Status Explorer — Active Products</title>
  <style>"##,
    );
    html.push_str(SHARED_WARM_PAPER_CSS);
    html.push_str(
        r##"
  </style>
</head>
<body>
  <header class="topbar">
    <div class="topbar-left">"##,
    );

    if let Some(ref logo_url) = overview.hydrogen_logo_url {
        let _ = write!(
            html,
            r##"<a href="/"><img class="hydrogen-mark" src="{}" alt="Hydrogen"></a>"##,
            escape_html(logo_url)
        );
    }

    let dup_suffix = if overview.duplicate_source_count > 0 {
        format!(" ({} dup)", overview.duplicate_source_count)
    } else {
        String::new()
    };
    let ai_suffix = if overview.replica_count > 0 {
        format!(" (+{} AI)", overview.replica_count)
    } else {
        String::new()
    };

    let _ = write!(
        html,
        r##"</div>
    <div class="topbar-center">
      <h1>Hydrogen Dashboard</h1>
    </div>
    <div class="topbar-right"></div>
  </header>
  <main>
    <div class="page-hero">
      <nav class="breadcrumbs"><span class="crumb-active">Products ({})</span></nav>
      <span class="hero-sep">&middot;</span>
      <div class="kpi-strip">
        <span class="kpi-pill">Sub-Products: {}</span>
        <span class="kpi-pill ok">Verified: {}{}</span>
        <span class="kpi-pill{}">Inbox: {}{}</span>
        <span class="kpi-pill{}">Comments: {}</span>
        <span class="kpi-pill{}">Tasks: {}</span>
      </div>
    </div>
"##,
        overview.active_product_count,
        overview.active_subject_count,
        overview.verified_question_count,
        escape_html(&ai_suffix),
        if overview.duplicate_source_count > 0 {
            " alert"
        } else {
            ""
        },
        overview.inbox_queued_count,
        escape_html(&dup_suffix),
        if overview.open_comment_count > 0 {
            " alert"
        } else {
            ""
        },
        overview.open_comment_count,
        if overview.pending_task_count > 0 {
            " alert"
        } else {
            ""
        },
        overview.pending_task_count,
    );

    if overview.products.is_empty() {
        html.push_str(
            r##"    <div class="empty-card">
      <strong>No active products discovered yet.</strong>
      <p>Initialize a vault with <code>hydrogen init-vault --product gyaraplus --subject maths</code> or drop a worksheet into <code>products/&lt;product&gt;/&lt;subject&gt;/inbox/</code>.</p>
    </div>
"##,
        );
    } else {
        html.push_str("    <div class=\"card-grid\">\n");
        for p in &overview.products {
            render_product_card(&mut html, p);
        }
        html.push_str("    </div>\n");
    }

    html.push_str("  </main>\n</body>\n</html>\n");
    html
}

fn render_product_card(html: &mut String, p: &ProductOverview) {
    let pct = compute_progress_pct(p.verified_question_count, p.expected_question_count);
    let exp_str = if p.expected_question_count > 0 {
        p.expected_question_count.to_string()
    } else {
        "?".to_string()
    };

    html.push_str("      <article class=\"explorer-card product-card\">\n        <div class=\"card-brand-center\">");
    if let Some(ref logo_url) = p.brand_logo_url {
        let _ = write!(
            html,
            r##"<a class="product-logo-box" href="{}"><img src="{}" alt="{}"></a>"##,
            escape_html(&p.product_url),
            escape_html(logo_url),
            escape_html(&p.title)
        );
    }
    let subj_word = if p.active_subject_count == 1 {
        "Sub-Product"
    } else {
        "Sub-Products"
    };
    let _ = write!(
        html,
        r##"<div>
            <a class="card-title-link" href="{}">{}</a>
            <div class="muted"><code>{}</code> &middot; <code>{}</code> &middot; <span class="badge progress">{} {subj_word}</span></div>
          </div>
        </div>
        <div class="chip-row" style="justify-content: center;">"##,
        escape_html(&p.product_url),
        escape_html(&p.title),
        escape_html(&p.product),
        escape_html(&p.product_code),
        p.active_subject_count,
    );

    for s in &p.subjects {
        let sub_exp = if s.expected_question_count > 0 {
            format!(
                "{}/{}",
                s.verified_question_count, s.expected_question_count
            )
        } else {
            format!("{} Qs", s.verified_question_count)
        };
        let _ = write!(
            html,
            r##"<a class="mineral-chip" style="border-left-color: {};" href="{}">{} <span class="muted">({sub_exp})</span></a>"##,
            escape_html(&s.mineral_color),
            escape_html(&s.status_url),
            escape_html(&s.subject),
        );
    }
    html.push_str("</div>\n");

    let _ = write!(
        html,
        r##"        <div class="progress-block">
          <div class="progress-label-row">
            <span>Progress (Verified / Expected)</span>
            <strong>{} / {} ({pct:.0}%)</strong>
          </div>
          <div class="progress-bar-track"><div class="progress-bar-fill" style="width: {pct:.1}%;"></div></div>
        </div>
        <div class="metrics-grid">
          <div>Papers: <strong>{}</strong></div>
          <div>Sliced Crops: <strong>{}</strong></div>
          <div>AI Replicas: <strong>{}</strong></div>
          <div>Topic Cards: <strong>{}</strong></div>
          <div>Inbox Queue: <strong>{}</strong></div>
          <div>Open Comments: <strong>{}</strong></div>
          <div>Pending Tasks: <strong>{}</strong></div>
          <div>Missing Slots: <strong>{}</strong></div>
        </div>
        <div class="card-footer">
          <div class="chip-row">"##,
        p.verified_question_count,
        exp_str,
        p.paper_count,
        p.sliced_question_count,
        p.replica_count,
        p.topic_card_count,
        p.inbox_queued_count,
        p.open_comment_count,
        p.pending_task_count,
        p.missing_question_count,
    );

    if p.duplicate_source_count > 0 {
        let _ = write!(
            html,
            r##"<span class="badge danger">{} Duplicate Source(s)</span>"##,
            p.duplicate_source_count
        );
    }
    if p.open_comment_count > 0 {
        let _ = write!(
            html,
            r##"<span class="badge danger">{} Open Comment(s)</span>"##,
            p.open_comment_count
        );
    }
    if p.pending_task_count > 0 {
        let _ = write!(
            html,
            r##"<span class="badge warn">{} Pending Task(s)</span>"##,
            p.pending_task_count
        );
    }
    if p.replica_count > 0 {
        let _ = write!(
            html,
            r##"<span class="badge ai">+{} AI Replica(s)</span>"##,
            p.replica_count
        );
    }

    let _ = write!(
        html,
        r##"</div>
          <a class="btn-primary" href="{}">Open {} &rarr;</a>
        </div>
      </article>
"##,
        escape_html(&p.product_url),
        escape_html(&p.product)
    );
}

/// Render `GET /products/{product}` — Carbon Warm-Paper Product Explorer Page (Sub-Product Cards).
pub fn render_product_explorer_html(product: &ProductOverview, has_hydrogen_logo: bool) -> String {
    let mut html = String::with_capacity(16_384);
    html.push_str(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Hydrogen Product Explorer — "##,
    );
    html.push_str(&escape_html(&product.product));
    html.push_str(
        r##"</title>
  <style>"##,
    );
    html.push_str(SHARED_WARM_PAPER_CSS);
    html.push_str(
        r##"
  </style>
</head>
<body>
  <header class="topbar">
    <div class="topbar-left">"##,
    );

    if has_hydrogen_logo {
        html.push_str(
            r##"<a href="/"><img class="hydrogen-mark" src="/assets/hydrogen-logo.png" alt="Hydrogen"></a>"##,
        );
    }
    html.push_str(
        r##"</div>
    <div class="topbar-center">
      <h1>Hydrogen Dashboard</h1>
    </div>
    <div class="topbar-right"></div>
  </header>
  <main>
    <div class="page-hero">"##,
    );
    if let Some(ref b_url) = product.brand_logo_url {
        let _ = write!(
            html,
            r##"<img class="brand-logo" src="{}" alt="{}">"##,
            escape_html(b_url),
            escape_html(&product.title)
        );
    }

    let prod_ai_suffix = if product.replica_count > 0 {
        format!(" (+{} AI)", product.replica_count)
    } else {
        String::new()
    };

    let _ = write!(
        html,
        r##"
      <nav class="breadcrumbs"><a class="crumb-pill" href="/">Products</a><span class="crumb-sep">&#9656;</span><span class="crumb-active">{}</span></nav>
      <span class="hero-sep">&middot;</span>
      <div class="kpi-strip">
        <span class="kpi-pill">Sub-Products: {}</span>
        <span class="kpi-pill">Papers: {}</span>
        <span class="kpi-pill ok">Verified: {}{}</span>
        <span class="kpi-pill{}">Inbox: {}</span>
        <span class="kpi-pill{}">Comments: {}</span>
        <span class="kpi-pill{}">Tasks: {}</span>
      </div>
    </div>
    <div class="card-grid">
"##,
        escape_html(&product.product),
        product.active_subject_count,
        product.paper_count,
        product.verified_question_count,
        escape_html(&prod_ai_suffix),
        if product.duplicate_source_count > 0 {
            " alert"
        } else {
            ""
        },
        product.inbox_queued_count,
        if product.open_comment_count > 0 {
            " alert"
        } else {
            ""
        },
        product.open_comment_count,
        if product.pending_task_count > 0 {
            " alert"
        } else {
            ""
        },
        product.pending_task_count,
    );

    for s in &product.subjects {
        render_subproduct_card(&mut html, s);
    }

    html.push_str("    </div>\n  </main>\n</body>\n</html>\n");
    html
}

fn render_subproduct_card(html: &mut String, s: &SubProductOverview) {
    let pct = compute_progress_pct(s.verified_question_count, s.expected_question_count);
    let exp_str = if s.expected_question_count > 0 {
        s.expected_question_count.to_string()
    } else {
        "?".to_string()
    };
    let bundle_badge_cls = if s.vault_initialized {
        "badge ok"
    } else {
        "badge"
    };

    let _ = write!(
        html,
        r##"      <article class="explorer-card subproduct-card" style="border-left-color: {};">
        <div class="card-header">
          <div>
            <a class="card-title-link" href="{}">{}</a>
            <div class="muted"><code>{}</code> &middot; <code>{}</code></div>
          </div>
          <span class="{bundle_badge_cls}"><code>{}</code></span>
        </div>
        <div class="progress-block">
          <div class="progress-label-row">
            <span>Verified / Expected</span>
            <strong>{} / {} ({pct:.0}%)</strong>
          </div>
          <div class="progress-bar-track"><div class="progress-bar-fill" style="width: {pct:.1}%;"></div></div>
        </div>
        <div class="metrics-grid">
          <div>Verified Qs: <strong>{}</strong></div>
          <div>Sliced Crops: <strong>{}</strong></div>
          <div>Missing Qs: <strong>{}</strong></div>
          <div>Replicas: <strong>{}</strong></div>
          <div>Papers / Topics: <strong>{} / {}</strong></div>
          <div>Inbox Queue: <strong>{}</strong></div>
          <div>Open Comments: <strong>{}</strong></div>
          <div>Pending Tasks: <strong>{}</strong></div>
        </div>
        <div class="card-footer">
          <form method="post" action="{}?redirect=product" style="margin:0;">
            <button type="submit" class="btn-secondary">Sync Vault</button>
          </form>
          <a class="btn-primary" href="{}">Status Dashboard &rarr;</a>
        </div>
      </article>
"##,
        escape_html(&s.mineral_color),
        escape_html(&s.status_url),
        escape_html(&s.title),
        escape_html(&s.course_id),
        escape_html(&s.subject),
        escape_html(&s.bundle_filename),
        s.verified_question_count,
        exp_str,
        s.verified_question_count,
        s.sliced_question_count,
        s.missing_question_count,
        s.replica_count + s.replica_paper_count,
        s.paper_count,
        s.topic_count,
        s.inbox_queued_count,
        s.open_comment_count,
        s.pending_task_count,
        escape_html(&s.sync_url),
        escape_html(&s.status_url),
    );
}

/// Render a clean Warm-Paper error page (`403`, `404`, `500`) with a link back to `/`.
pub fn render_error_html(status_code: u16, title: &str, detail: &str) -> String {
    let mut html = String::with_capacity(4_096);
    let _ = write!(
        html,
        r##"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Hydrogen {status_code} — {}</title>
  <style>{SHARED_WARM_PAPER_CSS}</style>
</head>
<body>
  <header class="topbar">
    <div class="topbar-left">
      <a href="/"><img class="hydrogen-mark" src="/assets/hydrogen-logo.png" alt="Hydrogen"></a>
    </div>
    <div class="topbar-center">
      <h1>Hydrogen Dashboard</h1>
    </div>
    <div class="topbar-right"></div>
  </header>
  <main>
    <div class="page-hero">
      <nav class="breadcrumbs"><a class="crumb-pill" href="/">Products</a><span class="crumb-sep">&#9656;</span><span class="crumb-active">HTTP {status_code} &middot; {}</span></nav>
    </div>
    <div class="empty-card">
      <p><strong>{}</strong></p>
      <p><a class="btn-primary" href="/">&larr; Back to Active Products</a></p>
    </div>
  </main>
</body>
</html>
"##,
        escape_html(title),
        escape_html(title),
        escape_html(detail)
    );
    html
}
