import { Code, ConnectError } from '@connectrpc/connect';
import { statusPageClient } from '../api/client';

function failure(error: unknown) {
  return { success: false as const, error: error instanceof ConnectError ? error.rawMessage : 'Unable to save the status page.' };
}

export async function createStatusPage(data: {
  name: string; slug: string; description?: string; monitorIds: string[];
  logoUrl?: string; isPublished?: boolean;
}) {
  try {
    const page = await statusPageClient.createStatusPage({
      title: data.name, slug: data.slug, description: data.description ?? '',
      monitorIds: data.monitorIds, logoUrl: data.logoUrl ?? '', isActive: data.isPublished ?? false,
    });
    return { success: true as const, page };
  } catch (error) { return failure(error); }
}

export async function updateStatusPage(id: string, data: {
  name?: string; slug?: string; description?: string; monitorIds?: string[];
  isPublished?: boolean; companyLogo?: string;
}) {
  try {
    const page = await statusPageClient.updateStatusPage({
      id, title: data.name, slug: data.slug, description: data.description,
      monitorIds: data.monitorIds, replaceMonitorIds: data.monitorIds !== undefined,
      isActive: data.isPublished, logoUrl: data.companyLogo,
    });
    return { success: true as const, page };
  } catch (error) { return failure(error); }
}

export async function publishStatusPage(id: string) { return updateStatusPage(id, { isPublished: true }); }

export async function deleteStatusPage(id: string) {
  try { await statusPageClient.deleteStatusPage({ id }); return { success: true as const }; }
  catch (error) { return failure(error); }
}

export async function validateSlug(slug: string, currentSlug?: string) {
  if (!/^[a-z0-9][a-z0-9-]{1,61}[a-z0-9]$/.test(slug)) {
    return { valid: false, message: 'Use 3 to 63 lowercase letters, numbers, or hyphens.' };
  }
  if (slug === currentSlug) return { valid: true };
  try {
    await statusPageClient.getStatusPage({ identifier: { case: 'slug', value: slug } });
    return { valid: false, message: 'This address is already in use.' };
  } catch (error) {
    if (error instanceof ConnectError && error.code === Code.NotFound) return { valid: true };
    return { valid: false, message: 'Unable to check this address. Try again shortly.' };
  }
}
