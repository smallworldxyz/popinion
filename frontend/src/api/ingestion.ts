
import api from '../api';

export interface IngestionParams {
    sources?: string[];
    keywords?: string[];
    limit?: number;
}

export interface IngestionResult {
    success: boolean;
    total_ingested: number;
    source_count: number;
    errors: string[];
    sample_data: any[];
}

// Axios interceptor returns the data object directly
export async function fetchIngestionDefaults(): Promise<{ success: boolean, data: string[] }> {
    return (await api.get('/ingestion/sources/defaults')) as any;
}

export async function startIngestion(params: IngestionParams): Promise<{ success: boolean, data: IngestionResult }> {
    return (await api.post('/ingestion/start', params)) as any;
}
