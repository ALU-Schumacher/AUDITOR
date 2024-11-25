{{/*
Expand the name of the chart.
*/}}
{{- define "auditor.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{- define "postgres.name" -}}
{{- cat (default .Chart.Name .Values.nameOverride) "-postgres" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Create a default fully qualified app name.
We truncate at 63 chars because some Kubernetes name fields are limited to this (by the DNS naming spec).
If release name contains chart name it will be used as a full name.
*/}}
{{- define "auditor.fullname" -}}
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
{{- define "auditor.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Common labels
*/}}
{{- define "auditor.labels" -}}
helm.sh/chart: {{ include "auditor.chart" . }}
{{ include "auditor.selectorLabels" . }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end }}

{{/*
Selector labels
*/}}
{{- define "auditor.selectorLabels" -}}
app.kubernetes.io/name: {{ include "auditor.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

{{/*
Create the name of the service account to use
*/}}
{{- define "auditor.serviceAccountName" -}}
{{- default (include "auditor.fullname" .) .Values.serviceAccount.name }}
{{- end }}

{{- define "auditor.clusterRoleName" -}}
{{- default (include "auditor.fullname" .) .Values.clusterRole.name }}
{{- end }}

{{- define "auditor.serviceName" -}}
{{- default (include "auditor.fullname" .) .Values.service.name }}
{{- end }}
