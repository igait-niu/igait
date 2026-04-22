{{/*
Standard Helm name/fullname/labels helpers.

`fullname` incorporates `.Release.Name` so per-PR previews (release name
`igait-pr-42`) don't collide with prod (`igait`). All resources reference
these so nothing hardcodes the string "igait".
*/}}

{{- define "igait.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{- define "igait.fullname" -}}
{{- if .Values.fullnameOverride -}}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" -}}
{{- else -}}
{{- $name := default .Chart.Name .Values.nameOverride -}}
{{- if contains $name .Release.Name -}}
{{- .Release.Name | trunc 63 | trimSuffix "-" -}}
{{- else -}}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" -}}
{{- end -}}
{{- end -}}
{{- end -}}

{{- define "igait.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{/* Standard labels applied to every resource. */}}
{{- define "igait.labels" -}}
helm.sh/chart: {{ include "igait.chart" . }}
{{ include "igait.selectorLabels" . }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end -}}

{{/* Base selector labels (stable — never include volatile fields here). */}}
{{- define "igait.selectorLabels" -}}
app.kubernetes.io/name: {{ include "igait.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end -}}

{{/* Resolves image reference for a given component.
     Usage: {{ include "igait.image" (dict "repo" "igait-backend" "ctx" .) }}
     ctx.Values.image.registry + / + repo + : + image.tag (or override). */}}
{{- define "igait.image" -}}
{{- $repo := .repo -}}
{{- $registry := .ctx.Values.image.registry -}}
{{- $tag := .tag | default .ctx.Values.image.tag -}}
{{- if not $tag -}}
{{- fail (printf "image.tag must be set (component=%s)" $repo) -}}
{{- end -}}
{{- printf "%s/%s:%s" $registry $repo $tag -}}
{{- end -}}
