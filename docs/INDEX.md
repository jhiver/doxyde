---
status: stable
last_updated: 2026-06-18
confidence: medium
source: "normalisation wikimd aios"
related: []
---

# Doxyde — Index de la documentation

Doxyde est un CMS moderne écrit en Rust, multi-sites (une base SQLite isolée par
domaine), pilotable par IA via MCP. Cet INDEX recense la documentation existante.

## Architecture

- [Architecture multi-site](../MULTISITE_ACTIVATION.md) — architecture multi-base de données (une SQLite par domaine).
- [Plan : templates & CSS par site](PLAN-per-site-templates.md) — personnalisation des templates, CSS et assets par site via MCP.

## Setup

- [README](../README.md) — présentation du projet et démarrage.
- [Guide de développement (CLAUDE.md)](../CLAUDE.md) — règles de développement, politique de langue, conventions du code.

## Runbooks

- [Plan de déploiement — migration multi-database](../migration-workspace/DEPLOYMENT_PLAN.md) — stratégie de migration de doxyde.com vers le multi-database.

## API

- [doxyde-tagger](../doxyde-tagger/README.md) — librairie d'auto-tagging HTML (port Rust de MKDoc::XML::Tagger).

## Misc

- [Guide de benchmarking](../BENCHMARKING.md) — instructions de benchmarking des performances.
- [Résultats de benchmark](../benchmark-results.md) — résultats de mesure de performance.
- [Couverture de tests — configuration](../CONFIGURATION_TEST_COVERAGE.md) — couverture de tests du système de configuration.
- [Audit sécurité — path traversal](../doxyde-web/SECURITY_AUDIT_PATH_TRAVERSAL.md) — audit de sécurité du web app (vulnérabilités path traversal).
