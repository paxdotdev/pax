import {ObjectPool} from "./object-pool";

export class ObjectManager {
    private pools: Map<string, ObjectPool<any>> = new Map();

    constructor(pools: { name: string; factory: (args?: any) => any; cleanUp: (item: any) => void }[]) {
        for (const pool of pools) {
            this.registerPool(pool.name, pool.factory, pool.cleanUp);
        }
    }

    registerPool<T>(name: string, factory: (args?: any) => T, reset: (item: T) => void) {
        this.pools.set(name, new ObjectPool(factory, reset));
    }

    getFromPool<T>(name: string, args?: any): T {
        const pool = this.pools.get(name);
        if (!pool) {
            throw new Error(`No pool registered with name: ${name}`);
        }
        return pool.get(args) as T;
    }

    returnToPool<T>(name: string, item: T) {
        const pool = this.pools.get(name);
        if (!pool) {
            throw new Error(`No pool registered with name: ${name}`);
        }
        pool.put(item);
    }
}