<script setup lang="ts">
/** Fenêtres partagées (store `dialogs`), rendues une fois par fenêtre qui en a besoin. */
import AddToWorkspacesDialog from "@/components/projects/AddToWorkspacesDialog.vue";
import NewProjectDialog from "@/components/projects/NewProjectDialog.vue";
import ConfirmProtectedDialog from "@/components/workbench/ConfirmProtectedDialog.vue";
import AddProjectsDialog from "@/components/workspaces/AddProjectsDialog.vue";
// Lot C - Import Bruno.
import ImportDialog from "@/components/import/ImportDialog.vue";
// Lot H - Réseau et authentification.
import PreferencesDialog from "@/components/preferences/PreferencesDialog.vue";
import ProjectSettingsDialog from "@/components/projects/ProjectSettingsDialog.vue";
</script>

<template>
  <NewProjectDialog />
  <AddToWorkspacesDialog />
  <AddProjectsDialog />
  <ConfirmProtectedDialog />
  <ImportDialog />
  <PreferencesDialog />
  <ProjectSettingsDialog />
</template>
