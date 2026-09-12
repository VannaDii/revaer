{{- define "revaer.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{- define "revaer.fullname" -}}
{{- if .Values.fullnameOverride -}}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" -}}
{{- else -}}
{{- $name := include "revaer.name" . -}}
{{- if contains $name .Release.Name -}}
{{- .Release.Name | trunc 63 | trimSuffix "-" -}}
{{- else -}}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" -}}
{{- end -}}
{{- end -}}
{{- end -}}

{{- define "revaer.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{- define "revaer.labels" -}}
helm.sh/chart: {{ include "revaer.chart" . }}
app.kubernetes.io/name: {{ include "revaer.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end -}}

{{- define "revaer.selectorLabels" -}}
app.kubernetes.io/name: {{ include "revaer.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end -}}

{{- define "revaer.serviceAccountName" -}}
{{- if .Values.serviceAccount.create -}}
{{- default (include "revaer.fullname" .) .Values.serviceAccount.name -}}
{{- else -}}
{{- default "default" .Values.serviceAccount.name -}}
{{- end -}}
{{- end -}}

{{- define "revaer.databaseSecretName" -}}
{{- if .Values.database.existingSecret -}}
{{- .Values.database.existingSecret -}}
{{- else -}}
{{- printf "%s-db" (include "revaer.fullname" .) -}}
{{- end -}}
{{- end -}}

{{- define "revaer.databaseSecretChecksum" -}}
{{- if and (not .Values.database.existingSecret) .Values.database.url -}}
{{ include (print $.Template.BasePath "/secret.yaml") . | sha256sum }}
{{- end -}}
{{- end -}}

{{- define "revaer.image" -}}
{{- printf "%s@%s" .Values.image.repository .Values.image.digest -}}
{{- end -}}

{{- define "revaer.validateCompliance" -}}
{{- range $key := list "image" "compliance" "nodeSelector" "podAnnotations" -}}
{{- if not (kindIs "map" (index $.Values $key)) -}}
{{- fail (printf "%s must be an object." $key) -}}
{{- end -}}
{{- end -}}
{{- range $key, $digest := dict "image.digest" .Values.image.digest "compliance.manifestDigest" .Values.compliance.manifestDigest -}}
{{- if not (kindIs "string" $digest) -}}
{{- fail (printf "%s must be a sha256: digest with exactly 64 lowercase hexadecimal characters." $key) -}}
{{- end -}}
{{- if not (regexMatch "^sha256:[a-f0-9]{64}$" $digest) -}}
{{- fail (printf "%s must be a sha256: digest with exactly 64 lowercase hexadecimal characters." $key) -}}
{{- end -}}
{{- end -}}
{{- if not (kindIs "string" .Values.image.tag) -}}
{{- fail "image.tag must be empty; image.digest is required." -}}
{{- end -}}
{{- if ne .Values.image.tag "" -}}
{{- fail "image.tag must be empty; image.digest is required." -}}
{{- end -}}
{{- if not (has .Values.image.architecture (list "amd64" "arm64")) -}}
{{- fail "image.architecture must be amd64 or arm64." -}}
{{- end -}}
{{- $claim := .Values.compliance.existingClaim -}}
{{- if not (kindIs "string" $claim) -}}
{{- fail "compliance.existingClaim must name an existing prepared PVC." -}}
{{- end -}}
{{- if or (gt (len $claim) 253) (not (regexMatch "^[a-z0-9]([-a-z0-9]*[a-z0-9])?(\\.[a-z0-9]([-a-z0-9]*[a-z0-9])?)*$" $claim)) -}}
{{- fail "compliance.existingClaim must name an existing prepared PVC." -}}
{{- end -}}
{{- if hasKey .Values.nodeSelector "kubernetes.io/arch" -}}
{{- if ne (index .Values.nodeSelector "kubernetes.io/arch") .Values.image.architecture -}}
{{- fail "nodeSelector kubernetes.io/arch conflicts with image.architecture." -}}
{{- end -}}
{{- end -}}
{{- if hasKey .Values.podAnnotations "checksum/compliance-manifest" -}}
{{- fail "podAnnotations checksum/compliance-manifest is reserved." -}}
{{- end -}}
{{- end -}}
