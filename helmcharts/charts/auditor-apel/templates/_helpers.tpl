{{/*
Expand the name of the chart.
*/}}
{{- define "auditor-apel.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Create a default fully qualified app name.
We truncate at 63 chars because some Kubernetes name fields are limited to this (by the DNS naming spec).
If release name contains chart name it will be used as a full name.
*/}}
{{- define "auditor-apel.fullname" -}}
{{- if .Values.fullnameOverride }}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- $name := default .Chart.Name .Values.nameOverride }}
{{- if contains $name .Release.Name }}
{{- .Release.Name | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" }}
{{- end }}
{{- end }}
{{- end }}

{{/*
Create chart name and version as used by the chart label.
*/}}
{{- define "auditor-apel.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Common labels
*/}}
{{- define "auditor-apel.labels" -}}
helm.sh/chart: {{ include "auditor-apel.chart" . }}
{{ include "auditor-apel.selectorLabels" . }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end }}

{{/*
Selector labels
*/}}
{{- define "auditor-apel.selectorLabels" -}}
app.kubernetes.io/name: {{ include "auditor-apel.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

{{/*
Create the fully qualified image path
*/}}
{{- define "auditor-apel.image" -}}
{{- $img := list .Values.registry .Values.repository | join "/" }}
{{- list $img (default .Chart.AppVersion .Values.tag) | join ":" }}
{{- end }}

{{/*
Create the name of the service account to use
*/}}
{{- define "auditor-apel.serviceAccountName" -}}
{{- default (include "auditor-apel.fullname" .) .Values.serviceAccount.name }}
{{- end }}

{{- define "auditor-apel.clusterRoleName" -}}
{{- default (include "auditor-apel.fullname" .) .Values.clusterRole.name }}
{{- end }}
