import { useMemo, useEffect, useState, useCallback } from 'react';
import Form from '@rjsf/core';
import type { IChangeEvent } from '@rjsf/core';
import { RJSFValidationError } from '@rjsf/utils';
import { customizeValidator } from '@rjsf/validator-ajv8';
import { useTranslation } from 'react-i18next';

import { Alert, AlertDescription } from '@/components/ui/alert';

// Create a custom validator that registers 'textarea' as a valid format
// This suppresses the "unknown format 'textarea' ignored in schema" warning
const validator = customizeValidator({
  customFormats: {
    textarea: () => true, // Always valid - textarea is a UI hint, not a validation constraint
  },
});
import { Card, CardContent } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Loader2 } from 'lucide-react';
import { shadcnTheme } from './rjsf';
import { BaseCodingAgent } from 'shared/types';
import type { AgentRuntimeCapabilities } from '@/lib/agentRuntimeCapabilities';
// Using custom shadcn/ui widgets instead of @rjsf/shadcn theme

interface ExecutorConfigFormProps {
  executor: BaseCodingAgent;
  value: unknown;
  runtimeCapabilities?: AgentRuntimeCapabilities | null;
  runtimeCapabilitiesStatus?: 'loading' | 'ready' | 'unavailable';
  onSubmit?: (formData: unknown) => void;
  onChange?: (formData: unknown) => void;
  onSave?: (formData: unknown) => Promise<void>;
  disabled?: boolean;
  isSaving?: boolean;
  isDirty?: boolean;
}

import schemas from 'virtual:executor-schemas';

export function ExecutorConfigForm({
  executor,
  value,
  runtimeCapabilities,
  runtimeCapabilitiesStatus = 'ready',
  onSubmit,
  onChange,
  onSave,
  disabled = false,
  isSaving = false,
  isDirty = false,
}: ExecutorConfigFormProps) {
  const { t } = useTranslation('settings');
  const [formData, setFormData] = useState<unknown>(value || {});
  const [validationErrors, setValidationErrors] = useState<
    RJSFValidationError[]
  >([]);

  const allowedCodexCollaborationModes = useMemo(() => {
    if (executor !== BaseCodingAgent.CODEX) {
      return null;
    }

    if (!runtimeCapabilities) {
      return null;
    }

    return new Set(
      runtimeCapabilities.collaboration_modes.flatMap((mode) =>
        mode.value ? [mode.value] : []
      )
    );
  }, [executor, runtimeCapabilities]);

  const sanitizeCodexFormData = useCallback(
    (rawFormData: unknown) => {
      if (
        executor !== BaseCodingAgent.CODEX ||
        !rawFormData ||
        typeof rawFormData !== 'object' ||
        Array.isArray(rawFormData)
      ) {
        return rawFormData;
      }

      const nextValue = { ...(rawFormData as Record<string, unknown>) };
      const selectedMode = nextValue.collaboration_mode;
      const shouldPruneForUnavailableRuntime =
        runtimeCapabilitiesStatus === 'unavailable' &&
        typeof selectedMode === 'string';
      const shouldPruneInvalidDiscoveredMode =
        typeof selectedMode === 'string' &&
        !!allowedCodexCollaborationModes &&
        !allowedCodexCollaborationModes.has(selectedMode);

      if (
        shouldPruneForUnavailableRuntime ||
        shouldPruneInvalidDiscoveredMode
      ) {
        delete nextValue.collaboration_mode;
      }

      return nextValue;
    },
    [allowedCodexCollaborationModes, executor, runtimeCapabilitiesStatus]
  );

  const schema = useMemo(() => {
    const baseSchema = schemas[executor];
    if (!baseSchema || executor !== BaseCodingAgent.CODEX) {
      return baseSchema;
    }

    const nextSchema = JSON.parse(JSON.stringify(baseSchema)) as Record<
      string,
      unknown
    >;
    const properties = (nextSchema.properties ?? {}) as Record<string, unknown>;
    delete properties.collaboration_mode;

    if (runtimeCapabilities?.models?.length) {
      const defaultModel =
        runtimeCapabilities.models.find((model) => model.is_default)
          ?.display_name ?? t('settings.agents.editor.runtimeDefaultModel');
      properties.model = {
        ...(properties.model as Record<string, unknown>),
        enum: [...runtimeCapabilities.models.map((model) => model.model), null],
        enumNames: [
          ...runtimeCapabilities.models.map(
            (model) => `${model.display_name} (${model.model})`
          ),
          t('settings.agents.editor.useRuntimeDefaultModel', {
            defaultModel,
          }),
        ],
      };
    }

    const discoveredModes = runtimeCapabilities?.collaboration_modes?.filter(
      (mode): mode is typeof mode & { value: string } => !!mode.value
    );
    if (discoveredModes?.length) {
      properties.collaboration_mode = {
        type: ['string', 'null'],
        title: t('settings.agents.editor.nativeCollaborationMode'),
        enum: [...discoveredModes.map((mode) => mode.value), null],
        enumNames: [
          ...discoveredModes.map((mode) => {
            const details = [mode.model, mode.reasoning_effort]
              .filter(Boolean)
              .join(' • ');
            return details
              ? t('settings.agents.editor.nativeCollaborationModeOption', {
                  label: mode.label,
                  details,
                })
              : mode.label;
          }),
          t('settings.agents.editor.noNativeCollaborationMode'),
        ],
      };
    }

    return nextSchema;
  }, [executor, runtimeCapabilities, t]);

  useEffect(() => {
    const nextFormData = sanitizeCodexFormData(value || {});

    setFormData(nextFormData);
    setValidationErrors([]);
  }, [sanitizeCodexFormData, value]);

  const handleChange = (event: IChangeEvent<unknown>) => {
    const newFormData = sanitizeCodexFormData(event.formData);
    setFormData(newFormData);
    if (onChange) {
      onChange(newFormData);
    }
  };

  const handleSubmit = async (event: IChangeEvent<unknown>) => {
    const submitData = sanitizeCodexFormData(event.formData);
    setValidationErrors([]);
    if (onSave) {
      await onSave(submitData);
    } else if (onSubmit) {
      onSubmit(submitData);
    }
  };

  const handleError = (errors: RJSFValidationError[]) => {
    setValidationErrors(errors);
  };

  if (!schema) {
    return (
      <Alert variant="destructive">
        <AlertDescription>
          {t('settings.agents.editor.schemaNotFound', { executor })}
        </AlertDescription>
      </Alert>
    );
  }

  return (
    <div className="space-y-8">
      <Card>
        <CardContent className="p-0">
          <Form
            schema={schema}
            formData={formData}
            onChange={handleChange}
            onSubmit={handleSubmit}
            onError={handleError}
            validator={validator}
            disabled={disabled}
            liveValidate
            showErrorList={false}
            widgets={shadcnTheme.widgets}
            templates={shadcnTheme.templates}
          >
            {onSave && (
              <div className="flex justify-end pt-4">
                <Button
                  type="submit"
                  disabled={!isDirty || validationErrors.length > 0 || isSaving}
                >
                  {isSaving && (
                    <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  )}
                  {t('settings.agents.save.button')}
                </Button>
              </div>
            )}
          </Form>
        </CardContent>
      </Card>

      {validationErrors.length > 0 && (
        <Alert variant="destructive">
          <AlertDescription>
            <ul className="list-disc list-inside space-y-1">
              {validationErrors.map((error, index) => (
                <li key={index}>
                  {error.property}: {error.message}
                </li>
              ))}
            </ul>
          </AlertDescription>
        </Alert>
      )}
    </div>
  );
}
