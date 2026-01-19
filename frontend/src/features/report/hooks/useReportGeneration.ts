import { ref, computed, onMounted, onUnmounted } from 'vue';
import { getAgentLog, getConsoleLog, getReportStatus } from '../../../api/report';

export interface ReportStatus {
    status: string;
    report_id?: string;
    file_path?: string;
    error?: string;
}

export function useReportGeneration(reportId: string | undefined) {
    const agentLogs = ref<any[]>([]);
    const consoleLogs = ref<string[]>([]);
    const reportStatus = ref<string>('initializing'); // initializing, running, completed, error
    const reportOutline = ref<any>(null);
    const generatedSections = ref<Record<string, string>>({});
    const isComplete = ref(false);

    // Tracking
    let pollingInterval: any = null;
    const agentLogLine = ref(0);
    const consoleLogLine = ref(0);
    const lastActivityTime = ref(Date.now());
    const POLLING_RATE = 2000;

    const startPolling = () => {
        if (!reportId || pollingInterval) return;

        pollingInterval = setInterval(async () => {
            try {
                await Promise.all([
                    pollAgentLogs(),
                    pollConsoleLogs(),
                    pollStatus()
                ]);

                // If complete, stop polling (or poll slower?)
                // Usually we want to keep polling console logs for a bit, or just stop.
                if (isComplete.value) {
                    stopPolling();
                    // Final fetch to ensure we got everything
                    await pollAgentLogs();
                }
            } catch (e) {
                console.error('Polling error:', e);
            }
        }, POLLING_RATE);
    };

    const stopPolling = () => {
        if (pollingInterval) {
            clearInterval(pollingInterval);
            pollingInterval = null;
        }
    };

    const pollAgentLogs = async () => {
        if (!reportId) return;
        try {
            const res: any = await getAgentLog(reportId, agentLogLine.value);
            if (res.success && res.data) {
                const newLogs = res.data.logs || [];
                if (newLogs.length > 0) {
                    lastActivityTime.value = Date.now();
                    agentLogs.value = [...agentLogs.value, ...newLogs];
                    agentLogLine.value = res.data.next_line; // Update cursor

                    processLogs(newLogs);
                }
            }
        } catch (e) {
            console.warn('Fetch agent log failed', e);
        }
    };

    const pollConsoleLogs = async () => {
        if (!reportId) return;
        try {
            const res: any = await getConsoleLog(reportId, consoleLogLine.value);
            if (res.success && res.data) {
                const newLogs = res.data.logs || [];
                if (newLogs.length > 0) {
                    consoleLogs.value = [...consoleLogs.value, ...newLogs];
                    consoleLogLine.value = res.data.next_line;
                }
            }
        } catch (e) {
            console.warn('Fetch console log failed', e);
        }
    };

    const pollStatus = async () => {
        if (!reportId) return;
        try {
            const res: any = await getReportStatus(reportId);
            if (res.success && res.data) {
                reportStatus.value = res.data.status;
                if (res.data.status === 'completed' || res.data.status === 'failed') {
                    isComplete.value = true;
                }
            }
        } catch (e) {
            // ignore
        }
    }

    const processLogs = (logs: any[]) => {
        logs.forEach(log => {
            // 1. Capture Outline
            if (log.action === 'planning_complete' && log.details?.outline) {
                reportOutline.value = log.details.outline;
            }

            // 2. Capture Generated Content
            // Update existing content logic (merging subsections etc)
            if (log.action === 'section_content' || log.action === 'subsection_content') {
                // Assuming log.details.content is the markdown
                // And we need to map it to section index.
                // Step4Report.vue used generatedSections[idx]
                // The log usually has `section_index`
                const idx = log.section_index;
                const content = log.details?.content || '';
                if (idx && content) {
                    // Append or Replace? Usually subsection adds to the section.
                    // But let's assume it's additive for now or replaced if it's the full section.
                    // Simplified logic: If we receive content for section X, store it.
                    // A robustness improvement is to handle subsections concatenation.

                    if (!generatedSections.value[idx]) {
                        generatedSections.value[idx] = content;
                    } else {
                        generatedSections.value[idx] += '\n\n' + content;
                    }
                }
            }

            if (log.action === 'report_complete') {
                isComplete.value = true;
            }
        });
    };

    onMounted(() => {
        if (reportId) startPolling();
    });

    onUnmounted(() => {
        stopPolling();
    });

    // Computed Stats for Executive Summary
    const stats = computed(() => {
        // derive from logs or parsed content
        // This is a placeholder. Real implementation might need to parse specific tool outputs from logs
        // to find "Graph Stats" or "Sentiment Analysis" results.
        return {
            status: reportStatus.value,
            totalLogs: agentLogs.value.length,
            completedSections: Object.keys(generatedSections.value).length,
            totalSections: reportOutline.value?.sections?.length || 0
        };
    });

    return {
        agentLogs,
        consoleLogs,
        reportStatus,
        reportOutline,
        generatedSections,
        isComplete,
        stats,
        pollingInterval // Exposed for debugging
    };
}
