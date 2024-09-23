{{/*
Expand the name of the chart.
*/}}
{{- define "auditor-collector.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Create a default fully qualified app name.
We truncate at 63 chars because some Kubernetes name fields are limited to this (by the DNS naming spec).
If release name contains chart name it will be used as a full name.
*/}}
{{- define "auditor-collector.fullname" -}}
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
{{- define "auditor-collector.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Common labels
*/}}
{{- define "auditor-collector.labels" -}}
helm.sh/chart: {{ include "auditor-collector.chart" . }}
{{ include "auditor-collector.selectorLabels" . }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end }}

{{/*
Selector labels
*/}}
{{- define "auditor-collector.selectorLabels" -}}
app.kubernetes.io/name: {{ include "auditor-collector.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

{{/*
Create the fully qualified image path
*/}}
{{- define "auditor-collector.image" -}}
{{- $img := list .Values.registry .Values.repository | join "/" }}
{{- list $img (default .Chart.AppVersion .Values.tag) | join ":" }}
{{- end }}

{{/*
Create the name of the service account to use
*/}}
{{- define "auditor-collector.serviceAccountName" -}}
{{- default (include "auditor-collector.fullname" .) .Values.serviceAccount.name }}
{{- end }}

{{- define "auditor-collector.clusterRoleName" -}}
{{- default (include "auditor-collector.fullname" .) .Values.clusterRole.name }}
{{- end }}
