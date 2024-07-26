export class ObjectPool<T> {
    private pool: T[] = [];
    private readonly factory: (args?: any) => T;
    private readonly cleanUp: (item: T) => void;

    constructor(factory: (args?: any) => T, cleanUp: (item: T) => void) {
        this.factory = factory;
        this.cleanUp = cleanUp;
    }

    get(args?: any): T {
        if (this.pool.length > 0) {
            return this.pool.pop() as T;
        }
        return this.factory(args);
    }

    put(item: T) {
        this.cleanUp(item);
        this.pool.push(item);
    }
}